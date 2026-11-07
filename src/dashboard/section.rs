//! Dashboard section container (`Section`), telemetry widget trait (`SectionWidget`),
//! and numbered menu option specification (`OptionSpec`).
//!
//! Directly ports `ref/dashboard/de_section.go`. Every section on a [`Page`](crate::dashboard::Page)
//! is either:
//! - [`SectionRole::Display`]: Hosts a [`SectionWidget`] that renders live telemetry
//!   or custom visual content inside a [`TabPanel`].
//! - [`SectionRole::Menu`]: Hosts a list of numbered [`OptionSpec`] entries rendered
//!   via [`MenuList`] inside a [`TabPanel`].

use crate::core::buffer::Buffer;
use crate::core::color::Color;
use crate::core::palette::ColorPalette;
use crate::core::rect::Rect;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{EventState, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext};
use crate::dashboard::sub_view::SubView;
use crate::widgets::menu_list::{MenuItem, MenuList};
use crate::widgets::panel::TabPanel;

/// Role of a [`Section`] on a dashboard page.
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::SectionRole;
///
/// assert_eq!(SectionRole::Display, SectionRole::Display);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionRole {
    /// Read-only or interactive telemetry display section backed by a [`SectionWidget`].
    Display,
    /// Numbered option menu section backed by a list of [`OptionSpec`] items.
    Menu,
}

/// Controls how focus is visually indicated on a [`Section`].
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::SectionFocusStyle;
///
/// assert_eq!(SectionFocusStyle::default(), SectionFocusStyle::Full);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SectionFocusStyle {
    /// Highlights only the outer [`TabPanel`] border when focused (ideal for telemetry graphs).
    Subtle,
    /// Highlights both the outer [`TabPanel`] border and the inner selection cursor (`❯`).
    #[default]
    Full,
}

/// Conditional visibility rule for a [`Section`].
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::SectionVisibility;
///
/// assert_eq!(SectionVisibility::default(), SectionVisibility::Always);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SectionVisibility {
    /// Always visible regardless of activity or terminal width.
    #[default]
    Always,
    /// Visible only when [`SectionWidget::is_active`] returns `true`.
    WhenActive,
    /// Visible only when the terminal width is at least `min_cols` columns.
    WhenWide {
        /// Minimum terminal width required for this section to appear.
        min_cols: u16,
    },
}

/// Outcome returned when a numbered [`OptionSpec`] is triggered.
pub enum OptionOutcome<B: DashboardBackend> {
    /// No overlay or notification change.
    None,
    /// Display a temporary toast notification in the footer bar.
    Toast {
        /// Notification message text.
        message: String,
        /// `true` if error/warning, `false` if success/info.
        is_error: bool,
    },
    /// Open an interactive modal overlay on top of the current page.
    OpenModal(Box<dyn Modal<B>>),
    /// Push a full-area drill-down sub-view onto the view stack.
    PushSubView(Box<dyn SubView<B>>),
    /// Request clean termination of the dashboard loop.
    Quit,
}

impl<B: DashboardBackend> core::fmt::Debug for OptionOutcome<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::None => f.write_str("OptionOutcome::None"),
            Self::Toast { message, is_error } => f
                .debug_struct("OptionOutcome::Toast")
                .field("message", message)
                .field("is_error", is_error)
                .finish(),
            Self::OpenModal(_) => f.write_str("OptionOutcome::OpenModal(..)"),
            Self::PushSubView(_) => f.write_str("OptionOutcome::PushSubView(..)"),
            Self::Quit => f.write_str("OptionOutcome::Quit"),
        }
    }
}

/// Type alias for dynamic badge callbacks on [`OptionSpec`].
pub type BadgeFn<B> = Box<dyn Fn(&B) -> (String, Option<Color>)>;

/// Type alias for dynamic enablement predicates on [`OptionSpec`].
pub type EnabledFn<B> = Box<dyn Fn(&B) -> bool>;

/// Type alias for option execution callbacks on [`OptionSpec`].
pub type ActionFn<B> = Box<dyn FnMut(&mut ModalContext<'_, B>) -> OptionOutcome<B>>;

/// Specification for a single numbered menu option (`[1]`..`[99]`) inside a [`SectionRole::Menu`] section.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::{DashboardBackend, OptionOutcome, OptionSpec};
///
/// struct MyBackend;
/// impl DashboardBackend for MyBackend {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let opt = OptionSpec::<MyBackend>::new(1, "Mount Encrypted Vault", |_ctx| {
///     OptionOutcome::Toast {
///         message: "Mounted vault!".to_string(),
///         is_error: false,
///     }
/// });
/// assert_eq!(opt.number(), 1);
/// assert_eq!(opt.label(), "Mount Encrypted Vault");
/// ```
pub struct OptionSpec<B: DashboardBackend> {
    number: u16,
    label: String,
    badge_fn: Option<BadgeFn<B>>,
    enabled_fn: Option<EnabledFn<B>>,
    action_fn: ActionFn<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for OptionSpec<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OptionSpec")
            .field("number", &self.number)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> OptionSpec<B> {
    /// Creates a new [`OptionSpec`] with shortcut `number`, static `label`, and `action` callback.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, OptionOutcome, OptionSpec};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let spec = OptionSpec::<App>::new(5, "Run Check", |_| OptionOutcome::None);
    /// assert_eq!(spec.number(), 5);
    /// ```
    #[must_use]
    pub fn new<F>(number: u16, label: impl Into<String>, action: F) -> Self
    where
        F: FnMut(&mut ModalContext<'_, B>) -> OptionOutcome<B> + 'static,
    {
        Self {
            number,
            label: label.into(),
            badge_fn: None,
            enabled_fn: None,
            action_fn: Box::new(action),
        }
    }

    /// Attaches a dynamic badge callback evaluated on every render pass.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, OptionOutcome, OptionSpec};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let spec = OptionSpec::<App>::new(1, "Status", |_| OptionOutcome::None)
    ///     .with_badge(|_| ("READY".to_string(), Some(Color::Green)));
    /// ```
    #[must_use]
    pub fn with_badge<F>(mut self, badge_fn: F) -> Self
    where
        F: Fn(&B) -> (String, Option<Color>) + 'static,
    {
        self.badge_fn = Some(Box::new(badge_fn));
        self
    }

    /// Attaches a dynamic enablement predicate evaluated on every render and key press.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, OptionOutcome, OptionSpec};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let spec = OptionSpec::<App>::new(1, "Unmount", |_| OptionOutcome::None)
    ///     .with_enabled(|_| false);
    /// ```
    #[must_use]
    pub fn with_enabled<F>(mut self, enabled_fn: F) -> Self
    where
        F: Fn(&B) -> bool + 'static,
    {
        self.enabled_fn = Some(Box::new(enabled_fn));
        self
    }

    /// Returns the numeric shortcut (`1..=99`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, OptionOutcome, OptionSpec};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let spec = OptionSpec::<App>::new(19, "Gaps", |_| OptionOutcome::None);
    /// assert_eq!(spec.number(), 19);
    /// ```
    #[must_use]
    #[inline]
    pub const fn number(&self) -> u16 {
        self.number
    }

    /// Returns the option label text.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, OptionOutcome, OptionSpec};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let spec = OptionSpec::<App>::new(19, "Gaps", |_| OptionOutcome::None);
    /// assert_eq!(spec.label(), "Gaps");
    /// ```
    #[must_use]
    #[inline]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Evaluates the dynamic badge text and optional color against `backend`.
    #[must_use]
    pub fn evaluate_badge(&self, backend: &B) -> (String, Option<Color>) {
        self.badge_fn
            .as_ref()
            .map_or_else(|| (String::new(), None), |f| f(backend))
    }

    /// Returns `true` if this option is currently enabled for `backend`.
    #[must_use]
    pub fn is_enabled(&self, backend: &B) -> bool {
        self.enabled_fn.as_ref().is_none_or(|f| f(backend))
    }

    /// Executes the option's action callback if enabled.
    pub fn trigger(&mut self, ctx: &mut ModalContext<'_, B>) -> OptionOutcome<B> {
        if !self.is_enabled(ctx.backend) {
            return OptionOutcome::Toast {
                message: format!("[{}] {} is currently disabled", self.number, self.label),
                is_error: true,
            };
        }
        (self.action_fn)(ctx)
    }
}

/// Trait implemented by display widgets inside a [`SectionRole::Display`] section.
pub trait SectionWidget<B: DashboardBackend> {
    /// Returns the title displayed in the section's top `TabPanel` notch.
    fn title(&self, backend: &B) -> String;

    /// Returns the subtitle displayed in the section's bottom `TabPanel` notch.
    fn subtitle(&self, _backend: &B) -> String {
        String::new()
    }

    /// Returns the right-aligned badge and optional color for the top `TabPanel` notch.
    fn badge(&self, _backend: &B) -> (String, Option<Color>) {
        (String::new(), None)
    }

    /// Minimum outer height in rows (including top/bottom borders).
    fn min_height(&self) -> u16 {
        5
    }

    /// Preferred outer height in rows (including top/bottom borders).
    fn preferred_height(&self) -> u16 {
        10
    }

    /// Returns `true` when this widget has active data (used by [`SectionVisibility::WhenActive`]).
    fn is_active(&self, _backend: &B) -> bool {
        true
    }

    /// Renders the widget's content into the `inner` area of its enclosing [`TabPanel`].
    fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        focused: bool,
        tty_mode: bool,
    );

    /// Handles a keyboard press when this section is focused.
    fn on_key(&mut self, _backend: &mut B, _key: KeyPress) -> EventState {
        EventState::NotConsumed
    }
}

/// A self-contained panel section on a [`Page`](crate::dashboard::Page).
pub struct Section<B: DashboardBackend> {
    role: SectionRole,
    title: String,
    subtitle: String,
    badge: String,
    min_height: u16,
    preferred_height: u16,
    visibility: SectionVisibility,
    focus_style: SectionFocusStyle,
    selectable: bool,
    widget: Option<Box<dyn SectionWidget<B>>>,
    options: Vec<OptionSpec<B>>,
    cursor: usize,
}

impl<B: DashboardBackend> core::fmt::Debug for Section<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Section")
            .field("role", &self.role)
            .field("title", &self.title)
            .field("selectable", &self.selectable)
            .field("options_len", &self.options.len())
            .field("cursor", &self.cursor)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> Section<B> {
    /// Creates a [`SectionRole::Display`] section wrapping a [`SectionWidget`].
    ///
    /// By default, display sections have `selectable = false` because they usually
    /// show informative visuals without user actions. Call [`Self::with_selectable`]
    /// with `true` if `Tab` should be able to focus this display section.
    #[must_use]
    pub fn display(widget: Box<dyn SectionWidget<B>>) -> Self {
        let min_h = widget.min_height().max(3);
        let pref_h = widget.preferred_height().max(min_h);
        Self {
            role: SectionRole::Display,
            title: String::new(),
            subtitle: String::new(),
            badge: String::new(),
            min_height: min_h,
            preferred_height: pref_h,
            visibility: SectionVisibility::Always,
            focus_style: SectionFocusStyle::Subtle,
            selectable: false,
            widget: Some(widget),
            options: Vec::new(),
            cursor: 0,
        }
    }

    /// Creates a [`SectionRole::Menu`] section with a title, subtitle, and numbered options.
    ///
    /// By default, menu sections have `selectable = true` so `Tab` focuses them.
    /// Call [`Self::with_selectable`] with `false` to exclude a section from `Tab` focus.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "option count fits in u16")]
    pub fn menu(
        title: impl Into<String>,
        subtitle: impl Into<String>,
        options: Vec<OptionSpec<B>>,
    ) -> Self {
        let pref_h = (options.len() as u16).saturating_add(2).max(5);
        Self {
            role: SectionRole::Menu,
            title: title.into(),
            subtitle: subtitle.into(),
            badge: String::new(),
            min_height: 4,
            preferred_height: pref_h,
            visibility: SectionVisibility::Always,
            focus_style: SectionFocusStyle::Full,
            selectable: true,
            widget: None,
            options,
            cursor: 0,
        }
    }

    /// Sets whether this section can be selected/focused when the user presses `Tab` (`true` or `false`).
    ///
    /// Informative widget sections with no user interaction should set `selectable` to `false`
    /// so `Tab` skips over them and only highlights interactive sections.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, Section};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let sec = Section::<App>::menu("Actions", "", Vec::new()).with_selectable(false);
    /// assert!(!sec.is_selectable());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Returns `true` if this section can be selected/focused via `Tab`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, Section};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let sec = Section::<App>::menu("Actions", "", Vec::new());
    /// assert!(sec.is_selectable());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_selectable(&self) -> bool {
        self.selectable
    }

    /// Sets the [`SectionVisibility`] policy.
    #[must_use]
    #[inline]
    pub const fn with_visibility(mut self, visibility: SectionVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets the [`SectionFocusStyle`].
    #[must_use]
    #[inline]
    pub const fn with_focus_style(mut self, focus_style: SectionFocusStyle) -> Self {
        self.focus_style = focus_style;
        self
    }

    /// Overrides `(min_height, preferred_height)` in outer rows.
    #[must_use]
    #[inline]
    pub const fn with_heights(mut self, min_height: u16, preferred_height: u16) -> Self {
        let min_h = if min_height < 3 { 3 } else { min_height };
        let pref_h = if preferred_height < min_h {
            min_h
        } else {
            preferred_height
        };
        self.min_height = min_h;
        self.preferred_height = pref_h;
        self
    }

    /// Appends a numbered [`OptionSpec`] to this section (used by [`AddonRegistry`](crate::dashboard::AddonRegistry)).
    #[expect(clippy::cast_possible_truncation, reason = "option count fits in u16")]
    pub fn push_option(&mut self, option: OptionSpec<B>) {
        self.options.push(option);
        self.options.sort_by_key(OptionSpec::number);
        let needed = (self.options.len() as u16).saturating_add(2);
        if needed > self.preferred_height {
            self.preferred_height = needed;
        }
    }

    /// Returns the [`SectionRole`].
    #[must_use]
    #[inline]
    pub const fn role(&self) -> SectionRole {
        self.role
    }

    /// Returns the minimum outer height in rows.
    #[must_use]
    #[inline]
    pub const fn min_height(&self) -> u16 {
        self.min_height
    }

    /// Returns the preferred outer height in rows.
    #[must_use]
    #[inline]
    pub const fn preferred_height(&self) -> u16 {
        self.preferred_height
    }

    /// Returns a slice of the section's numbered options.
    #[must_use]
    #[inline]
    pub fn options(&self) -> &[OptionSpec<B>] {
        &self.options
    }

    /// Returns a mutable slice of the section's numbered options.
    #[inline]
    pub fn options_mut(&mut self) -> &mut [OptionSpec<B>] {
        &mut self.options
    }

    /// Returns the current menu cursor index.
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Sets the menu cursor index, clamped to `0..options.len()`.
    #[inline]
    pub fn set_cursor(&mut self, index: usize) {
        if self.options.is_empty() {
            self.cursor = 0;
        } else {
            self.cursor = index.min(self.options.len() - 1);
        }
    }

    /// Moves the menu cursor down by 1 (wrapping around), returning `true` if moved.
    pub fn cursor_down(&mut self) -> bool {
        if self.options.is_empty() {
            return false;
        }
        self.cursor = (self.cursor + 1) % self.options.len();
        true
    }

    /// Moves the menu cursor up by 1 (wrapping around), returning `true` if moved.
    pub fn cursor_up(&mut self) -> bool {
        if self.options.is_empty() {
            return false;
        }
        self.cursor = (self.cursor + self.options.len() - 1) % self.options.len();
        true
    }

    /// Evaluates whether this section is currently visible given `backend` and `terminal_width`.
    #[must_use]
    pub fn is_visible(&self, backend: &B, terminal_width: u16) -> bool {
        match self.visibility {
            SectionVisibility::Always => true,
            SectionVisibility::WhenActive => {
                self.widget.as_ref().is_some_and(|w| w.is_active(backend))
            }
            SectionVisibility::WhenWide { min_cols } => terminal_width >= min_cols,
        }
    }

    /// Forwards a key press to the inner [`SectionWidget`] if present.
    pub fn on_widget_key(&mut self, backend: &mut B, key: KeyPress) -> EventState {
        self.widget
            .as_mut()
            .map_or(EventState::NotConsumed, |w| w.on_key(backend, key))
    }

    /// Renders the section's outer [`TabPanel`] and inner widget or [`MenuList`] into `area`.
    #[expect(clippy::too_many_arguments, reason = "internal section renderer")]
    pub fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        focused: bool,
        option_gap: u16,
        tty_mode: bool,
    ) {
        if area.is_empty() {
            return;
        }
        let title_cow = self
            .widget
            .as_ref()
            .map_or_else(|| self.title.clone(), |w| w.title(backend));
        let subtitle_cow = self
            .widget
            .as_ref()
            .map_or_else(|| self.subtitle.clone(), |w| w.subtitle(backend));
        let (badge_str, badge_color) = self
            .widget
            .as_ref()
            .map_or_else(|| (self.badge.clone(), None), |w| w.badge(backend));

        let is_focused = focused && self.selectable;
        let mut panel = TabPanel::new(&title_cow)
            .with_subtitle(&subtitle_cow)
            .with_badge(&badge_str)
            .with_focused(is_focused)
            .with_tty_mode(tty_mode);
        if let Some(bc) = badge_color {
            panel = panel.with_badge_color(bc);
        }

        let inner = panel.render(buffer, area, palette);
        if inner.is_empty() {
            return;
        }

        match self.role {
            SectionRole::Display => {
                if let Some(w) = &self.widget {
                    let inner_focused = is_focused && self.focus_style == SectionFocusStyle::Full;
                    w.render(backend, buffer, inner, palette, inner_focused, tty_mode);
                }
            }
            SectionRole::Menu => {
                // Evaluate dynamic badges and enablement for all options
                let evaluated_badges: Vec<(String, Option<Color>, bool)> = self
                    .options
                    .iter()
                    .map(|opt| {
                        let (b_txt, b_col) = opt.evaluate_badge(backend);
                        let en = opt.is_enabled(backend);
                        (b_txt, b_col, en)
                    })
                    .collect();

                let menu_items: Vec<MenuItem<'_>> = self
                    .options
                    .iter()
                    .zip(evaluated_badges.iter())
                    .map(|(opt, (b_txt, b_col, en))| {
                        let mut item = MenuItem::new(opt.number(), opt.label())
                            .with_badge(b_txt.as_str())
                            .with_enabled(*en);
                        if let Some(c) = *b_col {
                            item = item.with_badge_color(c);
                        }
                        item
                    })
                    .collect();

                let active_cursor = if is_focused && self.focus_style == SectionFocusStyle::Full {
                    Some(self.cursor.min(menu_items.len().saturating_sub(1)))
                } else {
                    None
                };

                MenuList::new(&menu_items)
                    .with_cursor(active_cursor)
                    .with_option_gap(option_gap)
                    .with_tty_mode(tty_mode)
                    .render(buffer, inner, palette);
            }
        }
    }
}
