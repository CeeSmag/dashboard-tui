//! Drill-down sub-page built from ordinary sections (`PageSubView`).
//!
//! A [`PageSubView`] is a full-area sub-view whose body is laid out exactly like a
//! dashboard [`Page`]: menu sections with numbered options and display widgets, arranged
//! by a [`SectionLayoutMode`]. Its options can toast, open modals, push deeper sub-views
//! or quit, just like options on a normal page, so a developer builds a settings or
//! detail screen with the same few lines used for the main pages.

use crate::core::buffer::Buffer;
use crate::core::palette::ColorPalette;
use crate::core::priority_strip::KeyHint;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::ModalContext;
use crate::dashboard::page::{Page, SectionLayoutMode};
use crate::dashboard::section::{OptionOutcome, Section, SectionRole};
use crate::dashboard::spacing::SpacingConfig;
use crate::dashboard::sub_view::{SubView, SubViewAction, SubViewResult};

/// Full-area drill-down page made of sections and numbered options.
///
/// Keys follow the engine's [`KeyBindings`](crate::dashboard::KeyBindings): up/down move the
/// cursor, left/right switch section, activate runs the option, digits jump to an option
/// number, option shortkeys work, and `Esc` / `b` / the cancel key go back.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::{
///     DashboardBackend, OptionOutcome, OptionSpec, PageSubView, Section,
/// };
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let mut network = Section::menu("Network", "1-2 run", Vec::new());
/// network.push_option(OptionSpec::new(1, "Allow LAN access", |_ctx| OptionOutcome::None));
/// network.push_option(OptionSpec::new(2, "Show address", |_ctx| OptionOutcome::None));
///
/// // Return this from any option: OptionOutcome::PushSubView(Box::new(view))
/// let view = PageSubView::<App>::new("Local Network").with_section(network);
/// assert_eq!(view.focused_section_index(), 0);
/// ```
pub struct PageSubView<B: DashboardBackend> {
    title: String,
    page: Page<B>,
    spacing: SpacingConfig,
    focused_section: usize,
    pending_number: Option<u16>,
}

impl<B: DashboardBackend> core::fmt::Debug for PageSubView<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageSubView")
            .field("title", &self.title)
            .field("focused_section", &self.focused_section)
            .field("pending_number", &self.pending_number)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> PageSubView<B> {
    /// Creates an empty sub-page titled `title`.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            page: Page::new(title.clone(), ""),
            title,
            spacing: SpacingConfig::default(),
            focused_section: usize::MAX,
            pending_number: None,
        }
    }

    /// Appends a section; the first selectable one (menus preferred) receives focus.
    #[must_use]
    pub fn with_section(mut self, section: Section<B>) -> Self {
        self.page.push_section(section);
        self.focused_section = self.default_focus();
        self
    }

    /// Sets how the sections are arranged (default: [`SectionLayoutMode::Auto`]).
    #[must_use]
    pub fn with_layout_mode(mut self, mode: SectionLayoutMode) -> Self {
        self.page.set_layout_mode(mode);
        self
    }

    /// Sets the 4-layer gaps used for this sub-page.
    #[must_use]
    pub const fn with_spacing(mut self, spacing: SpacingConfig) -> Self {
        self.spacing = spacing;
        self
    }

    /// Returns the index of the section that currently has focus.
    #[must_use]
    #[inline]
    pub const fn focused_section_index(&self) -> usize {
        if self.focused_section == usize::MAX {
            0
        } else {
            self.focused_section
        }
    }

    fn default_focus(&self) -> usize {
        let sections = self.page.sections();
        sections
            .iter()
            .position(|s| s.is_selectable() && s.role() == SectionRole::Menu)
            .or_else(|| sections.iter().position(Section::is_selectable))
            .unwrap_or(usize::MAX)
    }

    fn cycle_focus(&mut self, forward: bool) {
        let selectable: Vec<usize> = self
            .page
            .sections()
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_selectable())
            .map(|(idx, _)| idx)
            .collect();
        if selectable.is_empty() {
            return;
        }
        let current = selectable
            .iter()
            .position(|&idx| idx == self.focused_section)
            .unwrap_or(0);
        let count = selectable.len();
        let next = if forward {
            current.saturating_add(1) % count
        } else {
            current.saturating_add(count).saturating_sub(1) % count
        };
        self.focused_section = selectable.get(next).copied().unwrap_or(0);
    }

    fn find_option(
        &self,
        matches: impl Fn(&crate::dashboard::OptionSpec<B>) -> bool,
    ) -> Option<(usize, usize)> {
        self.page
            .sections()
            .iter()
            .enumerate()
            .find_map(|(sec_idx, sec)| {
                sec.options()
                    .iter()
                    .position(&matches)
                    .map(|opt_idx| (sec_idx, opt_idx))
            })
    }

    fn trigger(
        &mut self,
        sec_idx: usize,
        opt_idx: usize,
        ctx: &mut ModalContext<'_, B>,
    ) -> SubViewAction<B> {
        self.pending_number = None;
        self.focused_section = sec_idx;
        let Some(section) = self.page.sections_mut().get_mut(sec_idx) else {
            return SubViewAction::Result(SubViewResult::Continue);
        };
        section.set_cursor(opt_idx);
        let Some(option) = section.options_mut().get_mut(opt_idx) else {
            return SubViewAction::Result(SubViewResult::Continue);
        };
        SubViewAction::Outcome(option.trigger(ctx))
    }

    /// Feeds one typed digit into the multi-digit option-number jump.
    fn feed_digit(&mut self, digit: u16, ctx: &mut ModalContext<'_, B>) -> SubViewAction<B> {
        let typed = self
            .pending_number
            .unwrap_or(0)
            .saturating_mul(10)
            .saturating_add(digit);
        let has_longer_match =
            self.find_option(|o| o.number() > typed && starts_with_digits(o.number(), typed));
        let exact = self.find_option(|o| o.number() == typed);
        match (exact, has_longer_match) {
            (Some((sec, opt)), None) => self.trigger(sec, opt, ctx),
            (_, Some(_)) => {
                self.pending_number = Some(typed);
                SubViewAction::Result(SubViewResult::Continue)
            }
            (None, None) => {
                self.pending_number = None;
                SubViewAction::Outcome(OptionOutcome::None)
            }
        }
    }
}

/// Returns `true` if the decimal digits of `number` begin with the digits of `prefix`.
fn starts_with_digits(number: u16, prefix: u16) -> bool {
    let mut candidate = number;
    while candidate >= prefix && candidate > 0 {
        if candidate == prefix {
            return true;
        }
        candidate /= 10;
    }
    false
}

impl<B: DashboardBackend> SubView<B> for PageSubView<B> {
    fn title(&self) -> &str {
        &self.title
    }

    fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        self.page.render(
            backend,
            buffer,
            area,
            palette,
            self.spacing,
            self.focused_section,
            tty_mode,
        );
    }

    fn on_key(&mut self, _backend: &mut B, key: KeyPress) -> SubViewResult {
        // Without engine context only navigation-away is possible.
        match key.code() {
            KeyCode::Esc | KeyCode::Char('b') => SubViewResult::Pop,
            _ => SubViewResult::Continue,
        }
    }

    fn on_key_with_context(
        &mut self,
        key: KeyPress,
        ctx: &mut ModalContext<'_, B>,
    ) -> SubViewAction<B> {
        let keep_open = SubViewAction::Result(SubViewResult::Continue);
        if matches!(key.code(), KeyCode::Esc | KeyCode::Char('b')) && !key.has_ctrl() {
            return SubViewAction::Result(SubViewResult::Pop);
        }
        if ctx.keys.matches_cancel(key) {
            return keep_open;
        }

        // Pending multi-digit number: Enter commits, anything else is handled normally.
        if let Some(pending) = self.pending_number {
            if ctx.keys.matches_activate(key) {
                if let Some((sec, opt)) = self.find_option(|o| o.number() == pending) {
                    return self.trigger(sec, opt, ctx);
                }
                self.pending_number = None;
                return keep_open;
            }
            if !matches!(key.code(), KeyCode::Char(ch) if ch.is_ascii_digit()) {
                self.pending_number = None;
            }
        }

        if let Some((sec, opt)) = self.find_option(|o| o.shortkey().is_some_and(|k| key.matches(k)))
        {
            return self.trigger(sec, opt, ctx);
        }
        if let KeyCode::Char(ch) = key.code() {
            if !key.has_ctrl() && !key.has_alt() && ctx.keys.is_numeric_jump_enabled() {
                if let Some(digit) = ch.to_digit(10) {
                    return self.feed_digit(u16::try_from(digit).unwrap_or(0), ctx);
                }
            }
        }

        if ctx.keys.matches_next_section(key) {
            self.cycle_focus(true);
            return keep_open;
        }
        if ctx.keys.matches_prev_section(key) {
            self.cycle_focus(false);
            return keep_open;
        }

        let focused = self.focused_section;
        let Some(section) = self.page.sections_mut().get_mut(focused) else {
            return keep_open;
        };
        if section.role() == SectionRole::Display {
            let _ = section.on_widget_key(ctx.backend, key);
            return keep_open;
        }
        if ctx.keys.is_up(key) {
            let _ = section.cursor_up();
        } else if ctx.keys.is_down(key) {
            let _ = section.cursor_down();
        } else if ctx.keys.matches_activate(key) {
            let cursor = section.cursor();
            return self.trigger(focused, cursor, ctx);
        }
        keep_open
    }

    fn footer_hints(&self) -> Vec<KeyHint<'_>> {
        vec![
            KeyHint::new("↑↓", "select", Style::DEFAULT, 80),
            KeyHint::new("Enter", "activate", Style::DEFAULT, 90),
            KeyHint::new("1-9", "run option", Style::DEFAULT, 70),
            KeyHint::new("Esc", "back", Style::DEFAULT, 100),
        ]
    }
}
