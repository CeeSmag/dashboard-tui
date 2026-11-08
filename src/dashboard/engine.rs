//! Tier 1 Portable Dashboard Engine (`DashboardEngine`).
//!
//! Directly ports and enhances `Dashboard` from `ref/dashboard/de_dashboard.go`
//! and `de_keys.go`. Orchestrates:
//! - Multi-page (`F1`..`F12`, `[` / `]`) and multi-section (`Tab` / `Shift+Tab`) focus
//! - Multi-digit numeric shortcut jumping (`1`..`99` with `450ms` prefix window)
//! - Full-area drill-down [`SubView`] stack
//! - Centered [`Modal`] overlays with dimmed backdrop splicing
//! - 4-layer hierarchical [`SpacingConfig`] and live [`ColorPalette`] theming

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::core::buffer::Buffer;
use crate::core::coordinates::RowIndex;
use crate::core::palette::{ColorPalette, TerminalBackground};
use crate::core::priority_strip::KeyHint;
use crate::core::rect::{Rect, TerminalHeight};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::chrome::{render_footer_bar, render_header_bar};
use crate::dashboard::key_router::{EngineAction, IntoKeyPress, KeyBindings, KeyCode, KeyPress};
use crate::dashboard::modal::{
    Modal, ModalContext, ModalResult, splice_modal_overlay_with_backend,
};
use crate::dashboard::navigation::{DigitDispatch, PrefixJumpState};
use crate::dashboard::page::Page;
use crate::dashboard::section::{ActionFn, OptionOutcome, Section, SectionRole};
use crate::dashboard::spacing::SpacingConfig;
use crate::dashboard::sub_view::{SubView, SubViewAction, SubViewResult};
use crate::universal::ConfirmModal;

/// Duration (`2.5s`) a footer *hint* flash stays visible (a nudge such as "no shortcut for x").
const HINT_FLASH_DURATION: Duration = Duration::from_millis(2500);

/// Duration (`6s`) a footer *result* toast stays visible (something worth reading twice).
const RESULT_FLASH_DURATION: Duration = Duration::from_secs(6);

/// One footer flash message with its own lifetime.
#[derive(Debug)]
struct Flash {
    message: String,
    is_error: bool,
    shown_at: Instant,
    lifetime: Duration,
}

impl Flash {
    fn is_expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.shown_at) >= self.lifetime
    }
}

/// Callback producing a custom hint for an unmapped key (`None` stays silent).
type InvalidKeyHintFn = Box<dyn Fn(KeyPress) -> Option<String>>;

/// Tier 1 Portable Dashboard Engine parameterized over a [`DashboardBackend`].
pub struct DashboardEngine<B: DashboardBackend> {
    backend: B,
    pages: Vec<Page<B>>,
    active_page: usize,
    focused_sections: Vec<usize>,
    spacing: SpacingConfig,
    palette: ColorPalette,
    terminal_background: TerminalBackground,
    theme_index: usize,
    tty_mode: bool,
    keys: KeyBindings,
    key_actions: Vec<(KeyPress, ActionFn<B>)>,
    modal: Option<Box<dyn Modal<B>>>,
    sub_views: Vec<Box<dyn SubView<B>>>,
    toast: Option<Flash>,
    invalid_key_hint: Option<InvalidKeyHintFn>,
    invalid_key_hints_enabled: bool,
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
            terminal_background: TerminalBackground::Inherit,
            theme_index: 0,
            tty_mode: false,
            keys: KeyBindings::new(),
            key_actions: Vec::new(),
            modal: None,
            sub_views: Vec::new(),
            toast: None,
            invalid_key_hint: None,
            invalid_key_hints_enabled: true,
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

    /// Binds a global `key` (e.g. `"ctrl+x"`, `'o'`, `"F5"`) to an `action` that runs on the
    /// dashboard root (no modal or sub-view open). Return [`OptionOutcome::OpenModal`] to pop a dialog.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, PrioritySegment};
    /// use dashboard_tui::dashboard::{DashboardBackend, DashboardEngine, OptionOutcome};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let engine = DashboardEngine::new(App, Vec::new()).bind_key("ctrl+r", |_ctx| {
    ///     OptionOutcome::Toast { message: "refreshed".to_string(), is_error: false }
    /// });
    /// ```
    #[must_use]
    pub fn bind_key<F>(mut self, key: impl IntoKeyPress, action: F) -> Self
    where
        F: FnMut(&mut ModalContext<'_, B>) -> OptionOutcome<B> + 'static,
    {
        self.key_actions
            .push((key.into_key_press(), Box::new(action)));
        self
    }

    /// Binds a global `key` to a warning popup: `Enter` runs `on_confirm`, `q` / `Esc` cancels.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, PrioritySegment};
    /// use dashboard_tui::dashboard::{DashboardBackend, DashboardEngine};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let engine = DashboardEngine::new(App, Vec::new()).bind_confirmed_key(
    ///     "ctrl+x",
    ///     "Wipe Cache",
    ///     "This cannot be undone.",
    ///     |_app: &mut App| Ok("Cache wiped".to_string()),
    /// );
    /// ```
    #[must_use]
    pub fn bind_confirmed_key<F>(
        self,
        key: impl IntoKeyPress,
        title: impl Into<String>,
        warning: impl Into<String>,
        on_confirm: F,
    ) -> Self
    where
        F: Fn(&mut B) -> Result<String, String> + 'static,
    {
        let title = title.into();
        let warning = warning.into();
        let shared_confirm = Rc::new(on_confirm);
        self.bind_key(key, move |_ctx| {
            let confirm = Rc::clone(&shared_confirm);
            OptionOutcome::OpenModal(Box::new(ConfirmModal::warning(
                title.clone(),
                warning.clone(),
                move |backend: &mut B| confirm(backend),
            )))
        })
    }

    /// Chooses whether the dashboard paints the terminal background
    /// ([`TerminalBackground::Inherit`] keeps the terminal's own, the default;
    /// [`TerminalBackground::Theme`] follows the active theme;
    /// [`TerminalBackground::Fixed`] always uses one color).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, PrioritySegment, TerminalBackground};
    /// use dashboard_tui::dashboard::{DashboardBackend, DashboardEngine};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let engine = DashboardEngine::new(App, Vec::new())
    ///     .with_terminal_background(TerminalBackground::Theme);
    /// assert_eq!(engine.terminal_background(), TerminalBackground::Theme);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_terminal_background(mut self, background: TerminalBackground) -> Self {
        self.terminal_background = background;
        self
    }

    /// Changes the terminal background mode at runtime (for example from a settings toggle).
    #[inline]
    pub const fn set_terminal_background(&mut self, background: TerminalBackground) {
        self.terminal_background = background;
    }

    /// Returns the active terminal background mode.
    #[must_use]
    #[inline]
    pub const fn terminal_background(&self) -> TerminalBackground {
        self.terminal_background
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

    /// Returns `true` if a full-screen [`SubView`] is currently on top of the page.
    #[must_use]
    #[inline]
    pub fn has_sub_view(&self) -> bool {
        !self.sub_views.is_empty()
    }

    /// Returns the zero-based index of the focused section on the active page.
    #[must_use]
    pub fn focused_section(&self) -> usize {
        self.focused_sections
            .get(self.active_page)
            .copied()
            .unwrap_or(0)
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
    ///
    /// Result toasts stay visible for 6 seconds; see [`Self::show_hint`] for short nudges.
    pub fn show_toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some(Flash {
            message: message.into(),
            is_error,
            shown_at: Instant::now(),
            lifetime: RESULT_FLASH_DURATION,
        });
    }

    /// Displays a short-lived (2.5 seconds) hint flash in the bottom footer bar.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, PrioritySegment};
    /// use dashboard_tui::dashboard::{DashboardBackend, DashboardEngine};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let mut engine = DashboardEngine::new(App, Vec::new());
    /// engine.show_hint("Press ? for help");
    /// assert_eq!(engine.flash_message(), Some("Press ? for help"));
    /// ```
    pub fn show_hint(&mut self, message: impl Into<String>) {
        self.toast = Some(Flash {
            message: message.into(),
            is_error: true,
            shown_at: Instant::now(),
            lifetime: HINT_FLASH_DURATION,
        });
    }

    /// Returns the text of the footer flash currently shown, if any.
    #[must_use]
    pub fn flash_message(&self) -> Option<&str> {
        self.toast.as_ref().map(|flash| flash.message.as_str())
    }

    /// Replaces the default "No shortcut for …" hint with a custom one. Return `None`
    /// from `hint` to stay silent for a given key.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, PrioritySegment};
    /// use dashboard_tui::dashboard::{DashboardBackend, DashboardEngine};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let _engine = DashboardEngine::new(App, Vec::new())
    ///     .with_invalid_key_hint(|key| Some(format!("{} does nothing here", key.short_label().as_str())));
    /// ```
    #[must_use]
    pub fn with_invalid_key_hint<F>(mut self, hint: F) -> Self
    where
        F: Fn(KeyPress) -> Option<String> + 'static,
    {
        self.invalid_key_hint = Some(Box::new(hint));
        self
    }

    /// Turns the unmapped-key hint flash on or off (default: on).
    #[must_use]
    #[inline]
    pub const fn with_invalid_key_hints(mut self, enabled: bool) -> Self {
        self.invalid_key_hints_enabled = enabled;
        self
    }

    fn show_invalid_key_hint(&mut self, key: KeyPress) {
        if !self.invalid_key_hints_enabled {
            return;
        }
        let message = match &self.invalid_key_hint {
            Some(custom) => custom(key),
            None => Some(format!(
                "No shortcut for '{}'. Use {}, {} (open), {} (next page) or {} (quit).",
                key.short_label().as_str(),
                self.up_down_label(),
                self.keys.activate_press().short_label().as_str(),
                self.keys.next_page_press().short_label().as_str(),
                self.keys.quit_press().short_label().as_str(),
            )),
        };
        if let Some(text) = message {
            self.show_hint(text);
        }
    }

    fn up_down_label(&self) -> String {
        format!(
            "{}{}",
            self.keys.cursor_up_press().short_label().as_str(),
            self.keys.cursor_down_press().short_label().as_str()
        )
    }

    fn show_help_flash(&mut self) {
        let text = format!(
            "{} move · {} open · {} page · numbers jump · {} quit",
            self.up_down_label(),
            self.keys.activate_press().short_label().as_str(),
            self.keys.next_page_press().short_label().as_str(),
            self.keys.quit_press().short_label().as_str(),
        );
        self.show_toast(text, false);
    }

    /// Advances the engine tick counter, notifies `backend.on_tick()`, and
    /// commits any expired multi-digit prefix jump (`450ms`).
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.backend.on_tick();
        let now = Instant::now();
        if self
            .toast
            .as_ref()
            .is_some_and(|flash| flash.is_expired(now))
        {
            self.toast = None;
        }
        if let Some(digit) = self.prefix_jump.poll_expired(now) {
            let _ = self.execute_option_by_number(digit);
        }
    }

    /// Renders the complete dashboard frame (header bar, active page or sub-view,
    /// spliced modal overlay if open, and footer bar) into `area` inside `buffer`.
    pub fn render(&mut self, buffer: &mut Buffer, area: Rect) {
        self.render_frame(buffer, area);
        if let Some(background) = self.terminal_background.resolve(&self.palette) {
            buffer.fill_missing_background(area, background);
        }
    }

    fn render_frame(&mut self, buffer: &mut Buffer, area: Rect) {
        if area.is_empty() {
            return;
        }
        self.last_viewport_width = area.width().get();
        let now = Instant::now();
        if self
            .toast
            .as_ref()
            .is_some_and(|flash| flash.is_expired(now))
        {
            self.toast = None;
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
            splice_modal_overlay_with_backend(
                modal.as_ref(),
                &self.backend,
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
            .map(|flash| (flash.message.as_str(), flash.is_error));
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
        // Configured quit shortkey (^Q by default): terminate everything, optionally after
        // a warning popup (`KeyBindings::confirm_quit`).
        if self.keys.matches_quit(key) {
            if !self.keys.is_quit_confirmation_required() {
                return EngineAction::Quit;
            }
            if self.modal.is_none() {
                self.toast = None;
                self.open_modal(Box::new(ConfirmModal::quit(
                    "Quit",
                    "Are you sure you want to quit?",
                )));
                return EngineAction::Continue;
            }
        }
        // SIGINT (Ctrl+C): leave the UI immediately, like pressing the cancel key at the root.
        if key.has_ctrl() && key.code() == KeyCode::Char('c') {
            return EngineAction::Back;
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
                terminal_background: &mut self.terminal_background,
                tty_mode: self.tty_mode,
                keys: &self.keys,
            };
            match modal.on_key(key, &mut ctx) {
                ModalResult::Continue => {
                    if self.keys.matches_cancel(key) {
                        self.modal = None;
                    } else {
                        self.modal = Some(modal);
                    }
                }
                ModalResult::Consumed => self.modal = Some(modal),
                ModalResult::Close => {}
                ModalResult::Quit => return EngineAction::Quit,
                ModalResult::CloseWithToast { message, is_error } => {
                    self.show_toast(message, is_error);
                }
            }
            return EngineAction::Continue;
        }

        // 2. Active SubView on top of stack owns keys next
        if let Some(mut sub_view) = self.sub_views.pop() {
            let action = {
                let mut ctx = ModalContext {
                    backend: &mut self.backend,
                    spacing: &mut self.spacing,
                    palette: &mut self.palette,
                    theme_index: &mut self.theme_index,
                    terminal_background: &mut self.terminal_background,
                    tty_mode: self.tty_mode,
                    keys: &self.keys,
                };
                sub_view.on_key_with_context(key, &mut ctx)
            };
            match action {
                SubViewAction::Result(SubViewResult::Continue) => {
                    if !self.keys.matches_cancel(key) {
                        self.sub_views.push(sub_view);
                    }
                }
                SubViewAction::Result(SubViewResult::Pop) => {}
                SubViewAction::Result(SubViewResult::PopWithToast { message, is_error }) => {
                    self.show_toast(message, is_error);
                }
                SubViewAction::Outcome(outcome) => {
                    self.sub_views.push(sub_view);
                    return self.apply_option_outcome(outcome);
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

        // 4. Cancel key at dashboard root level: nothing is left to cancel, so go back.
        if self.keys.matches_cancel(key) {
            return EngineAction::Back;
        }

        // 4b. Developer-bound global keys (`bind_key` / `bind_confirmed_key`)
        if let Some(action_idx) = self
            .key_actions
            .iter()
            .position(|(bound, _)| bound.matches(key))
        {
            let outcome = {
                let mut ctx = ModalContext {
                    backend: &mut self.backend,
                    spacing: &mut self.spacing,
                    palette: &mut self.palette,
                    theme_index: &mut self.theme_index,
                    terminal_background: &mut self.terminal_background,
                    tty_mode: self.tty_mode,
                    keys: &self.keys,
                };
                (self.key_actions[action_idx].1)(&mut ctx)
            };
            return self.apply_option_outcome(outcome);
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
        if let Some(page) = self.pages.get(self.active_page) {
            let shortkey_target = page
                .sections()
                .iter()
                .enumerate()
                .find_map(|(sec_idx, sec)| {
                    sec.options()
                        .iter()
                        .position(|opt| opt.shortkey().is_some_and(|bound| bound.matches(key)))
                        .map(|opt_idx| (sec_idx, opt_idx))
                });
            if let Some((sec_idx, opt_idx)) = shortkey_target {
                return self.trigger_option_at(sec_idx, opt_idx);
            }
        }
        if key.code() == KeyCode::Char('?') && !key.has_ctrl() && !key.has_alt() {
            self.show_help_flash();
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
        if let Some(action) = self.route_to_focused_section(key) {
            return action;
        }
        self.show_invalid_key_hint(key);
        EngineAction::Continue
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

    fn route_to_focused_section(&mut self, key: KeyPress) -> Option<EngineAction> {
        let focused_sec = self
            .focused_sections
            .get(self.active_page)
            .copied()
            .unwrap_or(0);

        let page = self.pages.get_mut(self.active_page)?;
        let sec = page.sections_mut().get_mut(focused_sec)?;
        if !sec.is_selectable() {
            return None;
        }

        if sec.role() == SectionRole::Display
            && sec.on_widget_key(&mut self.backend, key).is_consumed()
        {
            return Some(EngineAction::Continue);
        }

        if sec.role() == SectionRole::Menu {
            if self.keys.is_up(key) {
                let _ = sec.cursor_up();
                return Some(EngineAction::Continue);
            } else if self.keys.is_down(key) {
                let _ = sec.cursor_down();
                return Some(EngineAction::Continue);
            } else if self.keys.matches_activate(key) {
                let cursor_idx = sec.cursor();
                if let Some(opt) = sec.options_mut().get_mut(cursor_idx) {
                    let outcome = {
                        let mut ctx = ModalContext {
                            backend: &mut self.backend,
                            spacing: &mut self.spacing,
                            palette: &mut self.palette,
                            theme_index: &mut self.theme_index,
                            terminal_background: &mut self.terminal_background,
                            tty_mode: self.tty_mode,
                            keys: &self.keys,
                        };
                        opt.trigger(&mut ctx)
                    };
                    return Some(self.apply_option_outcome(outcome));
                }
            }
        }
        None
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
        self.trigger_option_at(sec_idx, opt_idx)
    }

    fn trigger_option_at(&mut self, sec_idx: usize, opt_idx: usize) -> EngineAction {
        let Some(page) = self.pages.get_mut(self.active_page) else {
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
                terminal_background: &mut self.terminal_background,
                tty_mode: self.tty_mode,
                keys: &self.keys,
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
