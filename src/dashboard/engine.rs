//! Tier 1 Portable Dashboard Engine (`DashboardEngine`).
//!
//! Directly ports and enhances `Dashboard` from `ref/dashboard/de_dashboard.go`
//! and `de_keys.go`. Orchestrates:
//! - Multi-page (`F1`..`F12`, `[` / `]`) and multi-section (`Tab` / `Shift+Tab`) focus
//! - Multi-digit numeric shortcut jumping (`1`..`99` with `450ms` prefix window)
//! - Full-area drill-down [`SubView`] stack
//! - Centered [`Modal`] overlays with dimmed backdrop splicing
//! - 4-layer hierarchical [`SpacingConfig`] and live [`ColorPalette`] theming

use std::time::{Duration, Instant};

use crate::core::buffer::Buffer;
use crate::core::coordinates::RowIndex;
use crate::core::palette::ColorPalette;
use crate::core::priority_strip::KeyHint;
use crate::core::rect::{Rect, TerminalHeight};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::chrome::{render_footer_bar, render_header_bar};
use crate::dashboard::key_router::{EngineAction, KeyBindings, KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult, splice_modal_overlay};
use crate::dashboard::navigation::{DigitDispatch, PrefixJumpState};
use crate::dashboard::page::Page;
use crate::dashboard::section::{OptionOutcome, Section, SectionRole};
use crate::dashboard::spacing::SpacingConfig;
use crate::dashboard::sub_view::{SubView, SubViewResult};

/// Default duration (`4s`) that a footer toast notification stays visible.
const TOAST_DURATION: Duration = Duration::from_secs(4);

/// Tier 1 Portable Dashboard Engine parameterized over a [`DashboardBackend`].
pub struct DashboardEngine<B: DashboardBackend> {
    backend: B,
    pages: Vec<Page<B>>,
    active_page: usize,
    focused_sections: Vec<usize>,
    spacing: SpacingConfig,
    palette: ColorPalette,
    theme_index: usize,
    tty_mode: bool,
    keys: KeyBindings,
    modal: Option<Box<dyn Modal<B>>>,
    sub_views: Vec<Box<dyn SubView<B>>>,
    toast: Option<(String, bool, Instant)>,
    prefix_jump: PrefixJumpState,
    tick_count: usize,
    last_viewport_width: u16,
}

impl<B: DashboardBackend> core::fmt::Debug for DashboardEngine<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DashboardEngine")
            .field("active_page", &self.active_page)
            .field("pages_len", &self.pages.len())
            .field("spacing", &self.spacing)
            .field("theme_index", &self.theme_index)
            .field("tty_mode", &self.tty_mode)
            .field("keys", &self.keys)
            .field("has_modal", &self.modal.is_some())
            .field("sub_views_len", &self.sub_views.len())
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> DashboardEngine<B> {
    /// Creates a new [`DashboardEngine`] wrapping `backend` and `pages`.
    ///
    /// Automatically initializes each page's default section focus to its first
    /// selectable [`SectionRole::Menu`] section (or first selectable section of any role).
    #[must_use]
    pub fn new(backend: B, pages: Vec<Page<B>>) -> Self {
        let focused_sections = pages
            .iter()
            .map(|p| {
                p.sections()
                    .iter()
                    .position(|s| s.is_selectable() && s.role() == SectionRole::Menu)
                    .or_else(|| p.sections().iter().position(Section::is_selectable))
                    .unwrap_or(usize::MAX)
            })
            .collect();

        Self {
            backend,
            pages,
            active_page: 0,
            focused_sections,
            spacing: SpacingConfig::default(),
            palette: ColorPalette::DEFAULT,
            theme_index: 0,
            tty_mode: false,
            keys: KeyBindings::new(),
            modal: None,
            sub_views: Vec::new(),
            toast: None,
            prefix_jump: PrefixJumpState::new(),
            tick_count: 0,
            last_viewport_width: 120,
        }
    }

    /// Sets custom global shortkeys ([`KeyBindings`]) for this dashboard engine.
    #[must_use]
    #[inline]
    pub const fn with_keys(mut self, keys: KeyBindings) -> Self {
        self.keys = keys;
        self
    }

    /// Updates the global shortkeys ([`KeyBindings`]) in place.
    #[inline]
    pub const fn set_keys(&mut self, keys: KeyBindings) {
        self.keys = keys;
    }

    /// Returns a shared reference to the active [`KeyBindings`].
    #[must_use]
    #[inline]
    pub const fn keys(&self) -> &KeyBindings {
        &self.keys
    }

    /// Sets the initial 4-layer [`SpacingConfig`].
    #[must_use]
    #[inline]
    pub const fn with_spacing(mut self, spacing: SpacingConfig) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets the initial [`ColorPalette`] and preset `theme_index`.
    #[must_use]
    #[inline]
    pub const fn with_palette(mut self, palette: ColorPalette, theme_index: usize) -> Self {
        self.palette = palette;
        self.theme_index = theme_index;
        self
    }

    /// Configures whether Linux TTY safe glyph mode (`tty_mode`) is enabled.
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.set_tty_mode(tty_mode);
        self
    }

    /// Sets Linux TTY safe glyph mode (`tty_mode`) in place.
    #[inline]
    pub const fn set_tty_mode(&mut self, tty_mode: bool) {
        self.tty_mode = tty_mode;
        if tty_mode {
            self.palette = ColorPalette::TTY;
        }
    }

    /// Returns a shared reference to the host [`DashboardBackend`].
    #[must_use]
    #[inline]
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns a mutable reference to the host [`DashboardBackend`].
    #[inline]
    pub const fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Returns a shared slice of all pages in the dashboard.
    #[must_use]
    #[inline]
    pub fn pages(&self) -> &[Page<B>] {
        &self.pages
    }

    /// Returns a mutable slice of all pages in the dashboard.
    #[inline]
    pub fn pages_mut(&mut self) -> &mut [Page<B>] {
        &mut self.pages
    }

    /// Returns the zero-based index of the currently active page.
    #[must_use]
    #[inline]
    pub const fn active_page(&self) -> usize {
        self.active_page
    }

    /// Switches to `page_index` if within bounds, clearing any pending digit jump.
    pub fn set_active_page(&mut self, page_index: usize) {
        if page_index < self.pages.len() {
            self.active_page = page_index;
            self.prefix_jump.clear();
        }
    }

    /// Returns the active 4-layer [`SpacingConfig`].
    #[must_use]
    #[inline]
    pub const fn spacing(&self) -> SpacingConfig {
        self.spacing
    }

    /// Updates the 4-layer [`SpacingConfig`] in-place.
    #[inline]
    pub fn set_spacing(&mut self, spacing: SpacingConfig) {
        self.spacing = spacing;
    }

    /// Returns the active [`ColorPalette`].
    #[must_use]
    #[inline]
    pub const fn palette(&self) -> &ColorPalette {
        &self.palette
    }

    /// Updates the active [`ColorPalette`] and `theme_index` in-place.
    #[inline]
    pub fn set_palette(&mut self, palette: ColorPalette, theme_index: usize) {
        self.palette = palette;
        self.theme_index = theme_index;
    }

    /// Returns `true` if a modal overlay is currently open.
    #[must_use]
    #[inline]
    pub fn has_modal(&self) -> bool {
        self.modal.is_some()
    }

    /// Opens a [`Modal`] overlay on top of the active page.
    pub fn open_modal(&mut self, modal: Box<dyn Modal<B>>) {
        self.prefix_jump.clear();
        self.modal = Some(modal);
    }

    /// Closes any currently open [`Modal`] overlay.
    pub fn close_modal(&mut self) {
        self.modal = None;
    }

    /// Pushes a full-area drill-down [`SubView`] onto the view stack.
    pub fn push_sub_view(&mut self, view: Box<dyn SubView<B>>) {
        self.prefix_jump.clear();
        self.sub_views.push(view);
    }

    /// Pops the top [`SubView`] from the view stack, returning `true` if one was popped.
    pub fn pop_sub_view(&mut self) -> bool {
        self.sub_views.pop().is_some()
    }

    /// Displays a temporary toast notification in the bottom footer bar.
    pub fn show_toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some((message.into(), is_error, Instant::now()));
    }

    /// Advances the engine tick counter, notifies `backend.on_tick()`, and
    /// commits any expired multi-digit prefix jump (`450ms`).
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.backend.on_tick();
        let now = Instant::now();
        if let Some((_, _, stamped)) = &self.toast {
            if now.saturating_duration_since(*stamped) >= TOAST_DURATION {
                self.toast = None;
            }
        }
        if let Some(digit) = self.prefix_jump.poll_expired(now) {
            let _ = self.execute_option_by_number(digit);
        }
    }

    /// Renders the complete dashboard frame (header bar, active page or sub-view,
    /// spliced modal overlay if open, and footer bar) into `area` inside `buffer`.
    pub fn render(&mut self, buffer: &mut Buffer, area: Rect) {
        if area.is_empty() {
            return;
        }
        self.last_viewport_width = area.width().get();
        let now = Instant::now();
        if let Some((_, _, stamped)) = &self.toast {
            if now.saturating_duration_since(*stamped) >= TOAST_DURATION {
                self.toast = None;
            }
        }
        if let Some(digit) = self.prefix_jump.poll_expired(now) {
            let _ = self.execute_option_by_number(digit);
        }

        let w = area.width().get();
        let h = area.height().get();

        // Top 1-line header bar
        render_header_bar(
            &self.backend,
            &self.pages,
            self.active_page,
            buffer,
            area.x(),
            area.y(),
            w,
            &self.palette,
        );

        if h < 3 {
            return;
        }

        let body_area = Rect::new(
            area.x(),
            area.y().saturating_add(1),
            area.width(),
            TerminalHeight::new(h.saturating_sub(2)),
        );

        if let Some(sub_view) = self.sub_views.last() {
            sub_view.render(
                &self.backend,
                buffer,
                body_area,
                &self.palette,
                self.tty_mode,
            );
        } else if let Some(page) = self.pages.get(self.active_page) {
            let focused_sec = self
                .focused_sections
                .get(self.active_page)
                .copied()
                .unwrap_or(0);
            page.render(
                &self.backend,
                buffer,
                body_area,
                &self.palette,
                self.spacing,
                focused_sec,
                self.tty_mode,
            );
        }

        if let Some(modal) = &self.modal {
            splice_modal_overlay(
                modal.as_ref(),
                buffer,
                body_area,
                &self.palette,
                self.tty_mode,
            );
        }

        // Bottom 1-line footer bar
        let footer_row = RowIndex::new(area.bottom().get().saturating_sub(1));
        let hints = self.active_footer_hints();
        let active_toast = self
            .toast
            .as_ref()
            .map(|(msg, is_err, _)| (msg.as_str(), *is_err));
        render_footer_bar(
            buffer,
            area.x(),
            footer_row,
            w,
            &self.palette,
            &hints,
            active_toast,
            self.prefix_jump.pending_digit(),
            self.tty_mode,
        );
    }

    /// Processes a normalized [`KeyPress`] according to the 4-tier input priority
    /// hierarchy (`Modal` -> `SubView` -> `PrefixJump / Page / Section / Option`).
    #[expect(
        clippy::too_many_lines,
        reason = "single central 4-tier key dispatcher"
    )]
    pub fn handle_key(&mut self, key: KeyPress) -> EngineAction {
        // Immediate hard quit: SIGINT (Ctrl+C) or configured quit (^Q / Ctrl+Q)
        if (key.has_ctrl() && key.code() == KeyCode::Char('c')) || self.keys.matches_quit(key) {
            return EngineAction::Quit;
        }

        // Clear any visible toast on user key press
        self.toast = None;

        // 1. Modal overlay owns all keys when open
        if let Some(mut modal) = self.modal.take() {
            let mut ctx = ModalContext {
                backend: &mut self.backend,
                spacing: &mut self.spacing,
                palette: &mut self.palette,
                theme_index: &mut self.theme_index,
                tty_mode: self.tty_mode,
            };
            match modal.on_key(key, &mut ctx) {
                ModalResult::Continue => {
                    if self.keys.matches_cancel(key) {
                        self.modal = None;
                    } else {
                        self.modal = Some(modal);
                    }
                }
                ModalResult::Close => {}
                ModalResult::CloseWithToast { message, is_error } => {
                    self.show_toast(message, is_error);
                }
            }
            return EngineAction::Continue;
        }

        // 2. Active SubView on top of stack owns keys next
        if let Some(mut sub_view) = self.sub_views.pop() {
            match sub_view.on_key(&mut self.backend, key) {
                SubViewResult::Continue => {
                    if self.keys.matches_cancel(key) {
                        // SubView dismissed via global cancel ('q' / Esc)
                    } else {
                        self.sub_views.push(sub_view);
                    }
                }
                SubViewResult::Pop => {}
                SubViewResult::PopWithToast { message, is_error } => {
                    self.show_toast(message, is_error);
                }
            }
            return EngineAction::Continue;
        }

        // 3. Pending multi-digit jump handling
        if self.prefix_jump.has_pending() {
            if self.keys.matches_cancel(key) {
                self.prefix_jump.clear();
                return EngineAction::Continue;
            }
            if self.keys.matches_activate(key) {
                if let Some(num) = self.prefix_jump.commit_now() {
                    return self.execute_option_by_number(num);
                }
                return EngineAction::Continue;
            }
            if let KeyCode::Char(ch) = key.code()
                && !key.has_ctrl()
                && !key.has_alt()
                && ch.is_ascii_digit()
            {
                let digit = (ch as u16) - u16::from(b'0');
                let max_opt = self.active_page_max_option();
                if let DigitDispatch::FireImmediate(num) =
                    self.prefix_jump.feed_digit(digit, max_opt, Instant::now())
                {
                    return self.execute_option_by_number(num);
                }
                return EngineAction::Continue;
            }
            // Commit pending digit before processing non-digit navigation key
            if let Some(num) = self.prefix_jump.commit_now() {
                let _ = self.execute_option_by_number(num);
            }
        }

        // 4. Cancel key at dashboard root level (clears transient toast/state)
        if self.keys.matches_cancel(key) {
            return EngineAction::Continue;
        }

        // 5. Page & Section global navigation (using customizable `self.keys`)
        let code = key.code();
        if !key.has_ctrl() && !key.has_alt() {
            if let Some(page_idx) = self
                .pages
                .iter()
                .position(|p| KeyCode::from_hint(p.key_hint()) == Some(code))
            {
                self.set_active_page(page_idx);
                return EngineAction::Continue;
            }
            if let KeyCode::F(n) = code
                && (1..=12).contains(&n)
            {
                let target = usize::from(n - 1);
                self.set_active_page(target);
                return EngineAction::Continue;
            }
        }
        if self.keys.matches_prev_page(key) && !self.pages.is_empty() {
            let n = self.pages.len();
            self.set_active_page((self.active_page + n - 1) % n);
            return EngineAction::Continue;
        }
        if self.keys.matches_next_page(key) && !self.pages.is_empty() {
            let n = self.pages.len();
            self.set_active_page((self.active_page + 1) % n);
            return EngineAction::Continue;
        }
        if self.keys.matches_next_section(key) {
            self.cycle_section_focus(true);
            return EngineAction::Continue;
        }
        if self.keys.matches_prev_section(key) {
            self.cycle_section_focus(false);
            return EngineAction::Continue;
        }
        if self.keys.is_numeric_jump_enabled()
            && !key.has_ctrl()
            && !key.has_alt()
            && let KeyCode::Char(ch) = code
            && ch.is_ascii_digit()
        {
            let digit = (ch as u16) - u16::from(b'0');
            let max_opt = self.active_page_max_option();
            if let DigitDispatch::FireImmediate(num) =
                self.prefix_jump.feed_digit(digit, max_opt, Instant::now())
            {
                return self.execute_option_by_number(num);
            }
            return EngineAction::Continue;
        }

        // 6. Focused section routing
        self.route_to_focused_section(key)
    }

    fn active_page_max_option(&self) -> u16 {
        self.pages
            .get(self.active_page)
            .map_or(0, Page::max_option_number)
    }

    fn cycle_section_focus(&mut self, forward: bool) {
        let Some(page) = self.pages.get(self.active_page) else {
            return;
        };
        let term_w = self.last_viewport_width;
        let selectable_indices: Vec<usize> = page
            .sections()
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_selectable() && s.is_visible(&self.backend, term_w))
            .map(|(idx, _)| idx)
            .collect();

        if selectable_indices.is_empty() {
            return;
        }
        let current = self
            .focused_sections
            .get(self.active_page)
            .copied()
            .unwrap_or(0);
        let cur_pos = selectable_indices
            .iter()
            .position(|&idx| idx == current)
            .unwrap_or(0);
        let next_pos = if forward {
            (cur_pos + 1) % selectable_indices.len()
        } else {
            (cur_pos + selectable_indices.len() - 1) % selectable_indices.len()
        };
        if let Some(slot) = self.focused_sections.get_mut(self.active_page) {
            *slot = selectable_indices[next_pos];
        }
    }

    fn route_to_focused_section(&mut self, key: KeyPress) -> EngineAction {
        let focused_sec = self
            .focused_sections
            .get(self.active_page)
            .copied()
            .unwrap_or(0);

        let Some(page) = self.pages.get_mut(self.active_page) else {
            return EngineAction::Continue;
        };
        let Some(sec) = page.sections_mut().get_mut(focused_sec) else {
            return EngineAction::Continue;
        };
        if !sec.is_selectable() {
            return EngineAction::Continue;
        }

        if sec.role() == SectionRole::Display
            && sec.on_widget_key(&mut self.backend, key).is_consumed()
        {
            return EngineAction::Continue;
        }

        if sec.role() == SectionRole::Menu {
            if self.keys.is_up(key) {
                let _ = sec.cursor_up();
            } else if self.keys.is_down(key) {
                let _ = sec.cursor_down();
            } else if self.keys.matches_activate(key) {
                let cursor_idx = sec.cursor();
                if let Some(opt) = sec.options_mut().get_mut(cursor_idx) {
                    let outcome = {
                        let mut ctx = ModalContext {
                            backend: &mut self.backend,
                            spacing: &mut self.spacing,
                            palette: &mut self.palette,
                            theme_index: &mut self.theme_index,
                            tty_mode: self.tty_mode,
                        };
                        opt.trigger(&mut ctx)
                    };
                    return self.apply_option_outcome(outcome);
                }
            }
        }
        EngineAction::Continue
    }

    /// Finds the option with `number` on the active page, focuses its section and
    /// cursor, and triggers its action callback.
    pub fn execute_option_by_number(&mut self, number: u16) -> EngineAction {
        let Some(page) = self.pages.get_mut(self.active_page) else {
            return EngineAction::Continue;
        };

        let mut found: Option<(usize, usize)> = None;
        for (sec_idx, sec) in page.sections().iter().enumerate() {
            if let Some(opt_idx) = sec.options().iter().position(|o| o.number() == number) {
                found = Some((sec_idx, opt_idx));
                break;
            }
        }

        let Some((sec_idx, opt_idx)) = found else {
            self.show_toast(format!("No option [{number}] on this page"), true);
            return EngineAction::Continue;
        };

        if let Some(slot) = self.focused_sections.get_mut(self.active_page) {
            *slot = sec_idx;
        }
        let sec = &mut page.sections_mut()[sec_idx];
        sec.set_cursor(opt_idx);
        let outcome = {
            let mut ctx = ModalContext {
                backend: &mut self.backend,
                spacing: &mut self.spacing,
                palette: &mut self.palette,
                theme_index: &mut self.theme_index,
                tty_mode: self.tty_mode,
            };
            sec.options_mut()[opt_idx].trigger(&mut ctx)
        };
        self.apply_option_outcome(outcome)
    }

    fn apply_option_outcome(&mut self, outcome: OptionOutcome<B>) -> EngineAction {
        match outcome {
            OptionOutcome::None => EngineAction::Continue,
            OptionOutcome::Toast { message, is_error } => {
                self.show_toast(message, is_error);
                EngineAction::Continue
            }
            OptionOutcome::OpenModal(modal) => {
                self.open_modal(modal);
                EngineAction::Continue
            }
            OptionOutcome::PushSubView(view) => {
                self.push_sub_view(view);
                EngineAction::Continue
            }
            OptionOutcome::Quit => EngineAction::Quit,
        }
    }

    fn active_footer_hints(&self) -> Vec<KeyHint<'_>> {
        if let Some(sv) = self.sub_views.last() {
            return sv.footer_hints();
        }
        let key_style = self.palette.key_style();
        let max_opt = self.active_page_max_option();
        let mut hints = Vec::with_capacity(6);
        if self.keys.is_numeric_jump_enabled() {
            if max_opt > 9 {
                hints.push(KeyHint::new("1-99", "run option", key_style, 90));
            } else if max_opt > 0 {
                hints.push(KeyHint::new("1-9", "run option", key_style, 90));
            }
        }
        let up_down_lbl = format!(
            "{}{}",
            self.keys.cursor_up_press().short_label().as_str(),
            self.keys.cursor_down_press().short_label().as_str()
        );
        hints.push(KeyHint::new(up_down_lbl, "select", key_style, 80));
        hints.push(KeyHint::new(
            self.keys
                .activate_press()
                .short_label()
                .as_str()
                .to_string(),
            "activate",
            key_style,
            85,
        ));
        let sec_lbl = format!(
            "{}/{}",
            self.keys.next_section_press().short_label().as_str(),
            self.keys.prev_section_press().short_label().as_str()
        );
        hints.push(KeyHint::new(sec_lbl, "section", key_style, 60));
        if self.pages.len() > 1 {
            let page_lbl = self
                .keys
                .next_page_press()
                .short_label()
                .as_str()
                .to_string();
            hints.push(KeyHint::new(page_lbl, "page", key_style, 50));
        }
        hints.push(KeyHint::new(
            self.keys.quit_press().short_label().as_str().to_string(),
            "quit",
            key_style,
            100,
        ));
        hints
    }
}
