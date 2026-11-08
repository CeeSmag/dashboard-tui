//! Zero-edit declarative add-on registry (`AddonRegistry`, `AddonPageEntry`,
//! `AddonSectionEntry`, `AddonOptionEntry`).
//!
//! Directly ports `Assemble` from `ref/dashboard/de_layout.go`. Allows Tier 2
//! universal add-ons and Tier 3 project-specific add-ons to register pages,
//! sections, and numbered options declaratively by `(page_index, section_index)`,
//! assembling a complete [`DashboardEngine`] without modifying core engine code.

use std::time::{Duration, Instant};

use crate::backend::{Backend, CrosstermBackend};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::engine::DashboardEngine;
use crate::dashboard::key_router::{EngineAction, KeyBindings, KeyPress};
use crate::dashboard::page::{Page, SectionLayoutMode};
use crate::dashboard::section::{OptionSpec, Section, SectionWidget};
use crate::error::TuiError;
use crate::terminal::{
    TerminalSessionConfig, TerminalSessionGuard, install_panic_restoration_hook,
};

/// Declarative registration entry for a top-level [`Page`].
#[derive(Debug, Clone)]
pub struct AddonPageEntry {
    /// Zero-based page index (`0` = `F1`, `1` = `F2`, etc.).
    pub page_index: usize,
    /// Page title displayed in the top header tab bar.
    pub title: String,
    /// Function-key hint label (e.g., `"F1"`, `"F2"`).
    pub key_hint: String,
    /// Spatial layout mode for sections on this page.
    pub layout_mode: SectionLayoutMode,
}

impl AddonPageEntry {
    /// Creates a new [`AddonPageEntry`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{AddonPageEntry, SectionLayoutMode};
    ///
    /// let entry = AddonPageEntry::new(0, "Monitor", "F1", SectionLayoutMode::TwoTopOneBottom);
    /// assert_eq!(entry.page_index, 0);
    /// ```
    #[must_use]
    pub fn new(
        page_index: usize,
        title: impl Into<String>,
        key_hint: impl Into<String>,
        layout_mode: SectionLayoutMode,
    ) -> Self {
        Self {
            page_index,
            title: title.into(),
            key_hint: key_hint.into(),
            layout_mode,
        }
    }
}

/// Declarative registration entry attaching a [`Section`] to a target `page_index`.
pub struct AddonSectionEntry<B: DashboardBackend> {
    /// Target page index (`0`-based).
    pub page_index: usize,
    /// Section order index on that page (`0`-based).
    pub section_index: usize,
    /// Constructed [`Section`].
    pub section: Section<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for AddonSectionEntry<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddonSectionEntry")
            .field("page_index", &self.page_index)
            .field("section_index", &self.section_index)
            .field("section", &self.section)
            .finish()
    }
}

impl<B: DashboardBackend> AddonSectionEntry<B> {
    /// Creates a new [`AddonSectionEntry`].
    #[must_use]
    pub const fn new(page_index: usize, section_index: usize, section: Section<B>) -> Self {
        Self {
            page_index,
            section_index,
            section,
        }
    }

    /// Sets whether this section is selectable when pressing `Tab` (`true` or `false`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{AddonSectionEntry, DashboardBackend, Section};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let entry = AddonSectionEntry::new(0, 0, Section::<App>::menu("Menu", "", Vec::new()))
    ///     .with_selectable(false);
    /// assert!(!entry.section.is_selectable());
    /// ```
    #[must_use]
    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.section = self.section.with_selectable(selectable);
        self
    }
}

/// Declarative registration entry attaching a numbered [`OptionSpec`] to a
/// target `(page_index, section_index)` menu section.
pub struct AddonOptionEntry<B: DashboardBackend> {
    /// Target page index (`0`-based).
    pub page_index: usize,
    /// Target section index (`0`-based) on that page.
    pub section_index: usize,
    /// Numbered option specification.
    pub option: OptionSpec<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for AddonOptionEntry<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddonOptionEntry")
            .field("page_index", &self.page_index)
            .field("section_index", &self.section_index)
            .field("option", &self.option)
            .finish()
    }
}

impl<B: DashboardBackend> AddonOptionEntry<B> {
    /// Creates a new [`AddonOptionEntry`].
    #[must_use]
    pub const fn new(page_index: usize, section_index: usize, option: OptionSpec<B>) -> Self {
        Self {
            page_index,
            section_index,
            option,
        }
    }
}

/// Builder and assembler for wiring Tier 2 and Tier 3 add-ons into [`Page`] trees.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::{
///     AddonOptionEntry, AddonPageEntry, AddonRegistry, AddonSectionEntry, DashboardBackend,
///     OptionOutcome, OptionSpec, Section, SectionLayoutMode,
/// };
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("MOUNTED", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let mut reg = AddonRegistry::<App>::new();
/// reg.register_page(AddonPageEntry::new(0, "Monitor", "F1", SectionLayoutMode::Auto));
/// reg.register_section(AddonSectionEntry::new(
///     0,
///     0,
///     Section::menu("Quick Actions", "Enter run", Vec::new()),
/// ));
/// reg.register_option(AddonOptionEntry::new(
///     0,
///     0,
///     OptionSpec::new(1, "Mount Vault", |_| OptionOutcome::None),
/// ));
/// let pages = reg.assemble_pages();
/// assert_eq!(pages.len(), 1);
/// assert_eq!(pages[0].sections()[0].options().len(), 1);
/// ```
pub struct AddonRegistry<B: DashboardBackend> {
    pages: Vec<AddonPageEntry>,
    sections: Vec<AddonSectionEntry<B>>,
    options: Vec<AddonOptionEntry<B>>,
    keys: KeyBindings,
}

impl<B: DashboardBackend> core::fmt::Debug for AddonRegistry<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddonRegistry")
            .field("pages_len", &self.pages.len())
            .field("sections_len", &self.sections.len())
            .field("options_len", &self.options.len())
            .field("keys", &self.keys)
            .finish()
    }
}

impl<B: DashboardBackend> Default for AddonRegistry<B> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<B: DashboardBackend> AddonRegistry<B> {
    /// Creates an empty [`AddonRegistry`] with default [`KeyBindings`].
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            pages: Vec::new(),
            sections: Vec::new(),
            options: Vec::new(),
            keys: KeyBindings::new(),
        }
    }

    /// Configures custom global shortkeys ([`KeyBindings`]) for the dashboard in a single
    /// readable call inside `dashboard.rs`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{
    ///     AddonRegistry, DashboardBackend, KeyBindings, KeyCode, SectionLayoutMode,
    /// };
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let engine = AddonRegistry::<App>::new()
    ///     .with_keys(KeyBindings::default().quit('x').page_cycle('<', '>'))
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .build_engine(App);
    /// assert_eq!(engine.keys().quit_key(), KeyCode::Char('x'));
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_keys(mut self, keys: KeyBindings) -> Self {
        self.keys = keys;
        self
    }

    /// Registers a top-level page definition.
    pub fn register_page(&mut self, entry: AddonPageEntry) {
        self.pages.push(entry);
    }

    /// Registers a section attached to a page.
    pub fn register_section(&mut self, entry: AddonSectionEntry<B>) {
        self.sections.push(entry);
    }

    /// Registers a numbered menu option attached to a `(page_index, section_index)`.
    pub fn register_option(&mut self, entry: AddonOptionEntry<B>) {
        self.options.push(entry);
    }

    /// Fluent builder that registers a top-level page definition by `(page_index, title, key_hint, layout_mode)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{AddonRegistry, DashboardBackend, SectionLayoutMode};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let pages = AddonRegistry::<App>::new()
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .assemble_pages();
    /// assert_eq!(pages.len(), 1);
    /// ```
    #[must_use]
    pub fn with_page(
        mut self,
        page_index: usize,
        title: impl Into<String>,
        key_hint: impl Into<String>,
        layout_mode: SectionLayoutMode,
    ) -> Self {
        self.register_page(AddonPageEntry::new(
            page_index,
            title,
            key_hint,
            layout_mode,
        ));
        self
    }

    /// Fluent builder that registers a custom [`SectionWidget`] display panel at
    /// `(page_index, section_index)` with an explicit `selectable` (`true` or `false`) flag
    /// controlling whether pressing `Tab` can select this section.
    ///
    /// Set `selectable` to `false` for purely informative visual widgets that have no user
    /// actions, or `true` if the widget handles keyboard events when focused.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Color, ColorPalette, PrioritySegment, Rect};
    /// use tui_rs::dashboard::{AddonRegistry, DashboardBackend, SectionLayoutMode, SectionWidget};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    /// struct StatusBox;
    /// impl SectionWidget<App> for StatusBox {
    ///     fn title(&self, _: &App) -> String { "Status".into() }
    ///     fn render(&self, _: &App, _: &mut Buffer, _: Rect, _: &ColorPalette, _: bool, _: bool) {}
    /// }
    ///
    /// let pages = AddonRegistry::<App>::new()
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .with_display_section(0, 0, StatusBox, 5, 8, false)
    ///     .assemble_pages();
    /// assert!(!pages[0].sections()[0].is_selectable());
    /// ```
    #[must_use]
    pub fn with_display_section(
        mut self,
        page_index: usize,
        section_index: usize,
        widget: impl SectionWidget<B> + 'static,
        min_height: u16,
        preferred_height: u16,
        selectable: bool,
    ) -> Self {
        self.register_section(AddonSectionEntry::new(
            page_index,
            section_index,
            Section::display(Box::new(widget))
                .with_heights(min_height, preferred_height)
                .with_selectable(selectable),
        ));
        self
    }

    /// Fluent builder that registers an interactive menu section at `(page_index, section_index)`
    /// with an explicit `selectable` (`true` or `false`) flag controlling whether pressing `Tab`
    /// selects this section.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{AddonRegistry, DashboardBackend, SectionLayoutMode};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let pages = AddonRegistry::<App>::new()
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .with_menu_section(0, 0, "Quick Actions", "Enter run", 6, 10, true)
    ///     .assemble_pages();
    /// assert!(pages[0].sections()[0].is_selectable());
    /// ```
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "declarative fluent menu section builder"
    )]
    pub fn with_menu_section(
        mut self,
        page_index: usize,
        section_index: usize,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        min_height: u16,
        preferred_height: u16,
        selectable: bool,
    ) -> Self {
        self.register_section(AddonSectionEntry::new(
            page_index,
            section_index,
            Section::menu(title, subtitle, Vec::new())
                .with_heights(min_height, preferred_height)
                .with_selectable(selectable),
        ));
        self
    }

    /// Fluent builder that attaches an [`OptionSpec`] from an add-on module to `(page_index, section_index)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{
    ///     AddonRegistry, DashboardBackend, OptionOutcome, OptionSpec, SectionLayoutMode,
    /// };
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let engine = AddonRegistry::<App>::new()
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .with_menu_section(0, 0, "Actions", "1 run", 5, 8, true)
    ///     .with_option(0, 0, OptionSpec::new(1, "Mount", |_| OptionOutcome::None))
    ///     .build_engine(App);
    /// assert_eq!(engine.pages().len(), 1);
    /// ```
    #[must_use]
    pub fn with_option(
        mut self,
        page_index: usize,
        section_index: usize,
        option: OptionSpec<B>,
    ) -> Self {
        self.register_option(AddonOptionEntry::new(page_index, section_index, option));
        self
    }

    /// Assembles all registered pages, sections, and options and constructs a ready-to-run
    /// [`DashboardEngine`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{AddonRegistry, DashboardBackend, SectionLayoutMode};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let engine = AddonRegistry::<App>::new()
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .build_engine(App);
    /// assert_eq!(engine.pages().len(), 1);
    /// ```
    #[must_use]
    pub fn build_engine(self, backend: B) -> DashboardEngine<B> {
        let keys = self.keys;
        DashboardEngine::new(backend, self.assemble_pages()).with_keys(keys)
    }

    /// Assembles all registered pages, sections, and options into an ordered `Vec<Page<B>>`.
    #[must_use]
    pub fn assemble_pages(mut self) -> Vec<Page<B>> {
        self.pages.sort_by_key(|p| p.page_index);
        self.sections
            .sort_by_key(|s| (s.page_index, s.section_index));

        let mut built_pages: Vec<(usize, Page<B>)> = self
            .pages
            .into_iter()
            .map(|p| {
                (
                    p.page_index,
                    Page::new(p.title, p.key_hint).with_layout_mode(p.layout_mode),
                )
            })
            .collect();

        for sec_entry in self.sections {
            if let Some((_, page)) = built_pages
                .iter_mut()
                .find(|(p_idx, _)| *p_idx == sec_entry.page_index)
            {
                page.push_section(sec_entry.section);
            }
        }

        for opt_entry in self.options {
            if let Some((_, page)) = built_pages
                .iter_mut()
                .find(|(p_idx, _)| *p_idx == opt_entry.page_index)
            {
                if let Some(sec) = page.sections_mut().get_mut(opt_entry.section_index) {
                    sec.push_option(opt_entry.option);
                }
            }
        }

        built_pages.into_iter().map(|(_, p)| p).collect()
    }
}

impl<B: DashboardBackend> DashboardEngine<B> {
    /// Launches an interactive terminal session (installing the panic restoration hook,
    /// entering raw mode + alternate screen via [`TerminalSessionGuard`], auto-detecting
    /// Linux TTY fallback mode, handling terminal resizes, and ticking at `tick_rate`)
    /// until the user quits.
    ///
    /// This allows a project's `tui/dashboard.rs` file to remain a minimal declarative
    /// blueprint that wires add-ons from `tui/addons/` and launches in a single call.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if terminal initialization, event polling, or frame flushing fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{AddonRegistry, DashboardBackend, SectionLayoutMode};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "VaultDrive" }
    ///     fn status_badge(&self) -> (&str, Color) { ("MOUNTED", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let mut engine = AddonRegistry::<App>::new()
    ///     .with_page(0, "Monitor", "F1", SectionLayoutMode::Auto)
    ///     .build_engine(App);
    /// let reason = engine.run_interactive(Duration::from_millis(250))?;
    /// // `Back`: leave the UI; `Quit`: terminate the whole application.
    /// # let _ = reason;
    /// # Ok::<(), tui_rs::TuiError>(())
    /// ```
    pub fn run_interactive(&mut self, tick_rate: Duration) -> Result<EngineAction, TuiError> {
        install_panic_restoration_hook();
        let backend = CrosstermBackend::with_writer(std::io::stdout());
        let is_tty = backend.capabilities().is_tty_console();
        if is_tty {
            self.set_tty_mode(true);
        }
        let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
        let mut last_tick = Instant::now();

        loop {
            session.draw(|buffer, area| self.render(buffer, area))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if crossterm::event::poll(timeout)? {
                match crossterm::event::read()? {
                    crossterm::event::Event::Key(key_ev) => {
                        if let Some(kp) = KeyPress::from_crossterm(key_ev) {
                            match self.handle_key(kp) {
                                EngineAction::Continue => {}
                                exit => return Ok(exit),
                            }
                        }
                    }
                    crossterm::event::Event::Resize(_, _) => continue,
                    _ => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.tick();
                last_tick = Instant::now();
            }
        }
    }
}
