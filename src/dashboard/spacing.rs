//! 4-Layer hierarchical spacing system (`SpacingConfig`, `SpacingLayer`).
//!
//! Directly ports the 4-layer spacing architecture from `ref/dashboard/de_dashboard.go`
//! and `ref/dashboard/addons/universal/prompts.go`:
//! 1. **Layer 1 (`terminal_page_gap`, `0..=6`)**: Outer margin between the terminal
//!    edges and the page body (`1 unit = 1 row` top/bottom, `2 cols` left/right).
//! 2. **Layer 2 (`page_section_gap`, `0..=6`)**: Padding between the page frame and
//!    its child sections (`1 unit = 1 row` top/bottom, `2 cols` left/right).
//! 3. **Layer 3 (`section_gap`, `0..=10`)**: Gap between adjacent section panels
//!    (`1 unit = 1 row` vertical gap, `2 cols` horizontal gap).
//! 4. **Layer 4 (`section_option_gap`, `0..=6`)**: 4-directional inset around numbered
//!    menu options inside a menu section (`1 unit = 1 row` top/bottom, `2 cols` left/right).

use crate::core::rect::{Rect, Spacing4D};

/// Identifies one of the 4 configurable spacing layers in [`SpacingConfig`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::SpacingLayer;
///
/// assert_eq!(SpacingLayer::ALL.len(), 4);
/// assert_eq!(SpacingLayer::TerminalPage.max_value(), 6);
/// assert_eq!(SpacingLayer::BetweenSections.max_value(), 10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacingLayer {
    /// Layer 1: Terminal Edge ↔ Page (`0..=6`).
    TerminalPage,
    /// Layer 2: Page ↔ Sections (`0..=6`).
    PageSection,
    /// Layer 3: Section ↔ Section (`0..=10`).
    BetweenSections,
    /// Layer 4: Section ↔ Menu Options (`0..=6`).
    MenuOption,
}

impl SpacingLayer {
    /// All four spacing layers in top-down hierarchical order.
    pub const ALL: [Self; 4] = [
        Self::TerminalPage,
        Self::PageSection,
        Self::BetweenSections,
        Self::MenuOption,
    ];

    /// Returns the human-readable title of this spacing layer.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingLayer;
    ///
    /// assert_eq!(SpacingLayer::BetweenSections.label(), "Section <-> Section Gap");
    /// ```
    #[must_use]
    #[inline]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TerminalPage => "Terminal <-> Page Gap",
            Self::PageSection => "Page <-> Section Gap",
            Self::BetweenSections => "Section <-> Section Gap",
            Self::MenuOption => "Section <-> Option Gap",
        }
    }

    /// Returns a concise description of how this layer affects layout geometry.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingLayer;
    ///
    /// assert!(!SpacingLayer::MenuOption.description().is_empty());
    /// ```
    #[must_use]
    #[inline]
    pub const fn description(self) -> &'static str {
        match self {
            Self::TerminalPage => "Outer margin between terminal edges and the page (0-6)",
            Self::PageSection => "Inner padding between the page and its sections (0-6)",
            Self::BetweenSections => "Space between adjacent section panels on a page (0-10)",
            Self::MenuOption => "4-directional inset around options inside a menu section (0-6)",
        }
    }

    /// Returns the maximum allowed integer value for this spacing layer.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingLayer;
    ///
    /// assert_eq!(SpacingLayer::TerminalPage.max_value(), 6);
    /// assert_eq!(SpacingLayer::BetweenSections.max_value(), 10);
    /// ```
    #[must_use]
    #[inline]
    pub const fn max_value(self) -> u16 {
        match self {
            Self::TerminalPage | Self::PageSection | Self::MenuOption => 6,
            Self::BetweenSections => 10,
        }
    }
}

/// Complete 4-layer spacing configuration for a [`DashboardEngine`](crate::dashboard::DashboardEngine).
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::{SpacingConfig, SpacingLayer};
///
/// let mut spacing = SpacingConfig::default();
/// spacing.set(SpacingLayer::BetweenSections, 2);
/// assert_eq!(spacing.section_gap(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_field_names,
    reason = "4-layer spacing model intentionally uses *_gap domain terminology"
)]
pub struct SpacingConfig {
    terminal_page_gap: u16,
    page_section_gap: u16,
    section_gap: u16,
    section_option_gap: u16,
}

impl SpacingConfig {
    /// Creates a new [`SpacingConfig`] with all 4 gaps clamped to their respective ranges.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// let cfg = SpacingConfig::new(1, 1, 2, 1);
    /// assert_eq!(cfg.section_gap(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(
        terminal_page_gap: u16,
        page_section_gap: u16,
        section_gap: u16,
        section_option_gap: u16,
    ) -> Self {
        Self {
            terminal_page_gap: if terminal_page_gap > 6 {
                6
            } else {
                terminal_page_gap
            },
            page_section_gap: if page_section_gap > 6 {
                6
            } else {
                page_section_gap
            },
            section_gap: if section_gap > 10 { 10 } else { section_gap },
            section_option_gap: if section_option_gap > 6 {
                6
            } else {
                section_option_gap
            },
        }
    }

    /// Returns the value of the specified [`SpacingLayer`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{SpacingConfig, SpacingLayer};
    ///
    /// let cfg = SpacingConfig::new(1, 2, 3, 4);
    /// assert_eq!(cfg.get(SpacingLayer::BetweenSections), 3);
    /// ```
    #[must_use]
    #[inline]
    pub const fn get(&self, layer: SpacingLayer) -> u16 {
        match layer {
            SpacingLayer::TerminalPage => self.terminal_page_gap,
            SpacingLayer::PageSection => self.page_section_gap,
            SpacingLayer::BetweenSections => self.section_gap,
            SpacingLayer::MenuOption => self.section_option_gap,
        }
    }

    /// Sets the value of the specified [`SpacingLayer`], clamping to `0..=layer.max_value()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{SpacingConfig, SpacingLayer};
    ///
    /// let mut cfg = SpacingConfig::default();
    /// cfg.set(SpacingLayer::TerminalPage, 99);
    /// assert_eq!(cfg.terminal_page_gap(), 6);
    /// ```
    #[inline]
    pub fn set(&mut self, layer: SpacingLayer, value: u16) {
        let clamped = value.min(layer.max_value());
        match layer {
            SpacingLayer::TerminalPage => self.terminal_page_gap = clamped,
            SpacingLayer::PageSection => self.page_section_gap = clamped,
            SpacingLayer::BetweenSections => self.section_gap = clamped,
            SpacingLayer::MenuOption => self.section_option_gap = clamped,
        }
    }

    /// Resets all 4 spacing layers back to `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// let mut cfg = SpacingConfig::new(2, 2, 2, 2);
    /// cfg.reset();
    /// assert_eq!(cfg, SpacingConfig::default());
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns Layer 1 (`terminal_page_gap`, `0..=6`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// assert_eq!(SpacingConfig::default().terminal_page_gap(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn terminal_page_gap(&self) -> u16 {
        self.terminal_page_gap
    }

    /// Returns Layer 2 (`page_section_gap`, `0..=6`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// assert_eq!(SpacingConfig::default().page_section_gap(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn page_section_gap(&self) -> u16 {
        self.page_section_gap
    }

    /// Returns Layer 3 (`section_gap`, `0..=10`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// assert_eq!(SpacingConfig::default().section_gap(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn section_gap(&self) -> u16 {
        self.section_gap
    }

    /// Returns Layer 4 (`section_option_gap`, `0..=6`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// assert_eq!(SpacingConfig::default().section_option_gap(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn section_option_gap(&self) -> u16 {
        self.section_option_gap
    }

    /// Applies Layer 1 (`terminal_page_gap`) and Layer 2 (`page_section_gap`) insets
    /// to a raw page body [`Rect`], ensuring the inner region never shrinks below
    /// `20×4` cells on small terminals.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Rect, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::dashboard::SpacingConfig;
    ///
    /// let raw = Rect::from_size(TerminalWidth::new(100), TerminalHeight::new(30));
    /// let cfg = SpacingConfig::new(1, 1, 0, 0);
    /// let inset = cfg.apply_outer_insets(raw);
    /// assert_eq!(inset.width().get(), 92);
    /// assert_eq!(inset.height().get(), 26);
    /// ```
    #[must_use]
    pub fn apply_outer_insets(&self, body_area: Rect) -> Rect {
        let total_units = self.terminal_page_gap.saturating_add(self.page_section_gap);
        if total_units == 0 || body_area.is_empty() {
            return body_area;
        }
        let h = body_area.height().get();
        let w = body_area.width().get();

        let mut v_pad = total_units;
        if h.saturating_sub(v_pad.saturating_mul(2)) < 4 {
            v_pad = h.saturating_sub(4) / 2;
        }
        let mut h_pad = total_units.saturating_mul(2);
        if w.saturating_sub(h_pad.saturating_mul(2)) < 20 {
            h_pad = w.saturating_sub(20) / 2;
        }

        body_area.inner(Spacing4D::new(v_pad, h_pad, v_pad, h_pad))
    }
}
