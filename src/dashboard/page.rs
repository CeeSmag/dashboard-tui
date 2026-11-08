//! Multi-section page layout engine (`Page`, `SectionLayoutMode`).
//!
//! Directly ports `ref/dashboard/de_page.go`:
//! - Supports 6 explicit [`SectionLayoutMode`] presets:
//!   - [`SectionLayoutMode::Auto`]: Responsive layout (stacks Display sections
//!     above Menu sections; places 2 Display sections side-by-side when
//!     `width >= 96`).
//!   - [`SectionLayoutMode::Columns`]: Equal-width vertical columns (`│ A │ B │`).
//!   - [`SectionLayoutMode::Rows`]: Vertically stacked rows (`── A ── / ── B ──`).
//!   - [`SectionLayoutMode::OneTopTwoBottom`]: `1` full-width top + `2` side-by-side bottom.
//!   - [`SectionLayoutMode::TwoTopOneBottom`]: `2` side-by-side top + `1` full-width bottom.
//!   - [`SectionLayoutMode::Grid2x2`]: `2×2` four-panel grid.
//! - Honors Layer 3 (`section_gap`: `1 unit = 1 row` vertical gap, `2 cols` horizontal gap).
//! - Automatically hides overflow display sections when terminal height is too compact.

use crate::core::buffer::Buffer;
use crate::core::layout::split_equal_spans;
use crate::core::palette::ColorPalette;
use crate::core::rect::{ColumnIndex, Rect, TerminalHeight, TerminalWidth};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::section::{Section, SectionRole};
use crate::dashboard::spacing::SpacingConfig;

/// Preset spatial arrangement of visible sections on a [`Page`].
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::SectionLayoutMode;
///
/// assert_eq!(SectionLayoutMode::default(), SectionLayoutMode::Auto);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SectionLayoutMode {
    /// Responsive layout: Display panels on top (side-by-side when `width >= 96`),
    /// Menu panels stacked below.
    #[default]
    Auto,
    /// Side-by-side equal-width vertical columns (`│ S1 │ S2 │`).
    Columns,
    /// Vertically stacked full-width rows (`S1 / S2 / S3`).
    Rows,
    /// `1` full-width top panel + `2` side-by-side bottom panels (`3` sections).
    OneTopTwoBottom,
    /// `2` side-by-side top panels + `1` full-width bottom panel (`3` sections).
    TwoTopOneBottom,
    /// `2×2` symmetric grid (`4` sections).
    Grid2x2,
}

/// A resolved `(section_index, Rect)` placement computed by [`Page::resolve_layout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSectionRect {
    /// Index of the section within [`Page::sections`].
    pub section_index: usize,
    /// Assigned outer [`Rect`] in the terminal buffer.
    pub rect: Rect,
}

pub(crate) type TabBadgeFn<B> = Box<dyn Fn(&B) -> String>;

/// A top-level dashboard page switched via `F1`..`F12` (or `[` / `]`).
pub struct Page<B: DashboardBackend> {
    title: String,
    key_hint: String,
    layout_mode: SectionLayoutMode,
    sections: Vec<Section<B>>,
    tab_badge: Option<TabBadgeFn<B>>,
}

impl<B: DashboardBackend> core::fmt::Debug for Page<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Page")
            .field("title", &self.title)
            .field("key_hint", &self.key_hint)
            .field("layout_mode", &self.layout_mode)
            .field("sections_len", &self.sections.len())
            .field("has_tab_badge", &self.tab_badge.is_some())
            .finish()
    }
}

impl<B: DashboardBackend> Page<B> {
    /// Creates a new [`Page`] with the given `title` and `key_hint` (e.g., `"F1"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, Page, SectionLayoutMode};
    ///
    /// struct App;
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let page = Page::<App>::new("Monitor", "F1")
    ///     .with_layout_mode(SectionLayoutMode::TwoTopOneBottom);
    /// assert_eq!(page.title(), "Monitor");
    /// assert_eq!(page.key_hint(), "F1");
    /// ```
    #[must_use]
    pub fn new(title: impl Into<String>, key_hint: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            key_hint: key_hint.into(),
            layout_mode: SectionLayoutMode::Auto,
            sections: Vec::new(),
            tab_badge: None,
        }
    }

    /// Sets the [`SectionLayoutMode`] for this page.
    #[must_use]
    #[inline]
    pub const fn with_layout_mode(mut self, mode: SectionLayoutMode) -> Self {
        self.layout_mode = mode;
        self
    }

    /// Shows a live badge (e.g. `"●3"` or `"2 errors"`) beside this page's tab while it is
    /// the active page. Return an empty string to hide the badge.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, PrioritySegment};
    /// use tui_rs::dashboard::{DashboardBackend, Page};
    ///
    /// struct App { errors: usize }
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let page = Page::<App>::new("Logs", "F2").with_tab_badge(|app| {
    ///     if app.errors == 0 { String::new() } else { format!("{} errors", app.errors) }
    /// });
    /// assert_eq!(page.tab_badge(&App { errors: 2 }), "2 errors");
    /// assert_eq!(page.tab_badge(&App { errors: 0 }), "");
    /// ```
    #[must_use]
    pub fn with_tab_badge<F>(mut self, badge: F) -> Self
    where
        F: Fn(&B) -> String + 'static,
    {
        self.tab_badge = Some(Box::new(badge));
        self
    }

    pub(crate) fn set_tab_badge_boxed(&mut self, badge: TabBadgeFn<B>) {
        self.tab_badge = Some(badge);
    }

    /// Evaluates the tab badge text for `backend` (empty when none is set).
    #[must_use]
    pub fn tab_badge(&self, backend: &B) -> String {
        self.tab_badge
            .as_ref()
            .map_or_else(String::new, |f| f(backend))
    }

    /// Appends a [`Section`] to this page during builder construction.
    #[must_use]
    pub fn with_section(mut self, section: Section<B>) -> Self {
        self.sections.push(section);
        self
    }

    /// Appends a [`Section`] to this page in-place.
    pub fn push_section(&mut self, section: Section<B>) {
        self.sections.push(section);
    }

    /// Returns the page title.
    #[must_use]
    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the shortcut key label (`"F1"`, `"F2"`, etc.).
    #[must_use]
    #[inline]
    pub fn key_hint(&self) -> &str {
        &self.key_hint
    }

    /// Returns the configured [`SectionLayoutMode`].
    #[must_use]
    #[inline]
    pub const fn layout_mode(&self) -> SectionLayoutMode {
        self.layout_mode
    }

    /// Sets the [`SectionLayoutMode`] in-place.
    #[inline]
    pub fn set_layout_mode(&mut self, mode: SectionLayoutMode) {
        self.layout_mode = mode;
    }

    /// Returns a shared slice of all sections on this page.
    #[must_use]
    #[inline]
    pub fn sections(&self) -> &[Section<B>] {
        &self.sections
    }

    /// Returns a mutable slice of all sections on this page.
    #[inline]
    pub fn sections_mut(&mut self) -> &mut [Section<B>] {
        &mut self.sections
    }

    /// Returns the highest option number (`1..=99`) across all menu sections on this page.
    #[must_use]
    pub fn max_option_number(&self) -> u16 {
        let mut max_num = 0_u16;
        for sec in &self.sections {
            for opt in sec.options() {
                if opt.number() > max_num {
                    max_num = opt.number();
                }
            }
        }
        max_num
    }

    /// Resolves the [`Rect`] geometry for all visible sections on this page inside
    /// `page_body_area` (after applying Layer 1, Layer 2, and Layer 3 spacing).
    #[must_use]
    pub fn resolve_layout(
        &self,
        backend: &B,
        page_body_area: Rect,
        spacing: SpacingConfig,
    ) -> Vec<ResolvedSectionRect> {
        let body = spacing.apply_outer_insets(page_body_area);
        if body.is_empty() {
            return Vec::new();
        }
        let term_w = body.width().get();
        let visible: Vec<usize> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_visible(backend, term_w))
            .map(|(idx, _)| idx)
            .collect();

        if visible.is_empty() {
            return Vec::new();
        }
        if visible.len() == 1 {
            return vec![ResolvedSectionRect {
                section_index: visible[0],
                rect: body,
            }];
        }

        let v_gap = spacing.section_gap();
        let h_gap = spacing.section_gap().saturating_mul(2);

        match self.layout_mode {
            SectionLayoutMode::Columns => Self::layout_columns(&visible, body, h_gap),
            SectionLayoutMode::Rows => self.layout_rows(&visible, body, v_gap),
            SectionLayoutMode::OneTopTwoBottom if visible.len() >= 3 => {
                self.layout_split_2_1(&visible[..3], body, v_gap, h_gap, false)
            }
            SectionLayoutMode::TwoTopOneBottom if visible.len() >= 3 => {
                self.layout_split_2_1(&visible[..3], body, v_gap, h_gap, true)
            }
            SectionLayoutMode::Grid2x2 if visible.len() >= 4 => {
                self.layout_grid_2x2(&visible[..4], body, v_gap, h_gap)
            }
            _ => self.layout_auto(&visible, body, v_gap, h_gap),
        }
    }

    /// Renders all visible sections on this page into `page_body_area` inside `buffer`.
    #[expect(clippy::too_many_arguments, reason = "internal page renderer")]
    pub fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        page_body_area: Rect,
        palette: &ColorPalette,
        spacing: SpacingConfig,
        focused_section_index: usize,
        tty_mode: bool,
    ) {
        let resolved = self.resolve_layout(backend, page_body_area, spacing);
        for item in resolved {
            if let Some(sec) = self.sections.get(item.section_index) {
                sec.render(
                    backend,
                    buffer,
                    item.rect,
                    palette,
                    item.section_index == focused_section_index,
                    spacing.section_option_gap(),
                    tty_mode,
                );
            }
        }
    }

    fn column_spans(
        base_x: ColumnIndex,
        total_width: u16,
        count: usize,
        h_gap: u16,
    ) -> Vec<(ColumnIndex, TerminalWidth)> {
        let widths = split_equal_spans(total_width, h_gap, count);
        let mut cur_x = base_x;
        let mut out = Vec::with_capacity(widths.len());
        for w in widths {
            out.push((cur_x, TerminalWidth::new(w)));
            cur_x = cur_x.saturating_add(w).saturating_add(h_gap);
        }
        out
    }

    fn layout_columns(visible: &[usize], body: Rect, h_gap: u16) -> Vec<ResolvedSectionRect> {
        let spans = Self::column_spans(body.x(), body.width().get(), visible.len(), h_gap);
        visible
            .iter()
            .zip(spans)
            .map(|(&sec_idx, (col_x, col_w))| ResolvedSectionRect {
                section_index: sec_idx,
                rect: Rect::new(col_x, body.y(), col_w, body.height()),
            })
            .collect()
    }

    #[expect(clippy::cast_possible_truncation, reason = "section count fits in u16")]
    fn layout_rows(&self, visible: &[usize], body: Rect, v_gap: u16) -> Vec<ResolvedSectionRect> {
        let mut active = visible.to_vec();
        // Drop trailing Display sections from the bottom if height is too tight
        while active.len() > 1 {
            let total_gaps = v_gap.saturating_mul((active.len().saturating_sub(1)) as u16);
            let min_sum: u16 = active
                .iter()
                .map(|&idx| self.sections[idx].min_height())
                .sum();
            if min_sum.saturating_add(total_gaps) <= body.height().get() {
                break;
            }
            if let Some(drop_pos) = active
                .iter()
                .rposition(|&idx| self.sections[idx].role() == SectionRole::Display)
            {
                active.remove(drop_pos);
            } else {
                break;
            }
        }

        let heights = self.distribute_heights(&active, body.height().get(), v_gap);
        let mut out = Vec::with_capacity(active.len());
        let mut cur_y = body.y();
        for (&sec_idx, h) in active.iter().zip(heights) {
            out.push(ResolvedSectionRect {
                section_index: sec_idx,
                rect: Rect::new(body.x(), cur_y, body.width(), TerminalHeight::new(h)),
            });
            cur_y = cur_y.saturating_add(h).saturating_add(v_gap);
        }
        out
    }

    fn layout_split_2_1(
        &self,
        vis3: &[usize],
        body: Rect,
        v_gap: u16,
        h_gap: u16,
        two_on_top: bool,
    ) -> Vec<ResolvedSectionRect> {
        if body.width().get() < 76 || body.height().get() < 10 {
            return self.layout_rows(vis3, body, v_gap);
        }
        let avail_h = body.height().get().saturating_sub(v_gap);
        let (top_pref, top_min, bot_pref, bot_min) = if two_on_top {
            let tp = self.sections[vis3[0]]
                .preferred_height()
                .max(self.sections[vis3[1]].preferred_height());
            let tm = self.sections[vis3[0]]
                .min_height()
                .max(self.sections[vis3[1]].min_height());
            let bp = self.sections[vis3[2]].preferred_height();
            let bm = self.sections[vis3[2]].min_height();
            (tp, tm, bp, bm)
        } else {
            let tp = self.sections[vis3[0]].preferred_height();
            let tm = self.sections[vis3[0]].min_height();
            let bp = self.sections[vis3[1]]
                .preferred_height()
                .max(self.sections[vis3[2]].preferred_height());
            let bm = self.sections[vis3[1]]
                .min_height()
                .max(self.sections[vis3[2]].min_height());
            (tp, tm, bp, bm)
        };

        let mut top_h = top_pref.min(avail_h.saturating_sub(bot_min));
        top_h = top_h.max(top_min).min(avail_h.saturating_sub(3));
        let preferred_bot = bot_pref.min(avail_h.saturating_sub(top_h)).max(bot_min);
        let extra = avail_h.saturating_sub(top_h.saturating_add(preferred_bot));
        if extra > 0 {
            top_h = top_h.saturating_add(extra / 2);
        }
        let bot_h = avail_h.saturating_sub(top_h);
        if top_h < 3 || bot_h < 3 {
            return self.layout_rows(vis3, body, v_gap);
        }

        let cols = Self::column_spans(body.x(), body.width().get(), 2, h_gap);
        let top_y = body.y();
        let bot_y = top_y.saturating_add(top_h).saturating_add(v_gap);

        if two_on_top {
            vec![
                ResolvedSectionRect {
                    section_index: vis3[0],
                    rect: Rect::new(cols[0].0, top_y, cols[0].1, TerminalHeight::new(top_h)),
                },
                ResolvedSectionRect {
                    section_index: vis3[1],
                    rect: Rect::new(cols[1].0, top_y, cols[1].1, TerminalHeight::new(top_h)),
                },
                ResolvedSectionRect {
                    section_index: vis3[2],
                    rect: Rect::new(body.x(), bot_y, body.width(), TerminalHeight::new(bot_h)),
                },
            ]
        } else {
            vec![
                ResolvedSectionRect {
                    section_index: vis3[0],
                    rect: Rect::new(body.x(), top_y, body.width(), TerminalHeight::new(top_h)),
                },
                ResolvedSectionRect {
                    section_index: vis3[1],
                    rect: Rect::new(cols[0].0, bot_y, cols[0].1, TerminalHeight::new(bot_h)),
                },
                ResolvedSectionRect {
                    section_index: vis3[2],
                    rect: Rect::new(cols[1].0, bot_y, cols[1].1, TerminalHeight::new(bot_h)),
                },
            ]
        }
    }

    fn layout_grid_2x2(
        &self,
        vis4: &[usize],
        body: Rect,
        v_gap: u16,
        h_gap: u16,
    ) -> Vec<ResolvedSectionRect> {
        if body.width().get() < 76 || body.height().get() < 10 {
            return self.layout_rows(vis4, body, v_gap);
        }
        let avail_h = body.height().get().saturating_sub(v_gap);
        let top_h = avail_h / 2;
        let bot_h = avail_h.saturating_sub(top_h);
        let cols = Self::column_spans(body.x(), body.width().get(), 2, h_gap);
        let top_y = body.y();
        let bot_y = top_y.saturating_add(top_h).saturating_add(v_gap);

        vec![
            ResolvedSectionRect {
                section_index: vis4[0],
                rect: Rect::new(cols[0].0, top_y, cols[0].1, TerminalHeight::new(top_h)),
            },
            ResolvedSectionRect {
                section_index: vis4[1],
                rect: Rect::new(cols[1].0, top_y, cols[1].1, TerminalHeight::new(top_h)),
            },
            ResolvedSectionRect {
                section_index: vis4[2],
                rect: Rect::new(cols[0].0, bot_y, cols[0].1, TerminalHeight::new(bot_h)),
            },
            ResolvedSectionRect {
                section_index: vis4[3],
                rect: Rect::new(cols[1].0, bot_y, cols[1].1, TerminalHeight::new(bot_h)),
            },
        ]
    }

    fn layout_auto(
        &self,
        visible: &[usize],
        body: Rect,
        v_gap: u16,
        h_gap: u16,
    ) -> Vec<ResolvedSectionRect> {
        let mut displays = Vec::new();
        let mut menus = Vec::new();
        for &idx in visible {
            match self.sections[idx].role() {
                SectionRole::Display => displays.push(idx),
                SectionRole::Menu => menus.push(idx),
            }
        }

        // Wide terminal with 2 displays + 1 menu -> TwoTopOneBottom
        if body.width().get() >= 96 && displays.len() == 2 && menus.len() == 1 {
            let ordered = [displays[0], displays[1], menus[0]];
            return self.layout_split_2_1(&ordered, body, v_gap, h_gap, true);
        }
        // Wide terminal with 2 displays and no menu -> side-by-side columns
        if body.width().get() >= 96 && displays.len() == 2 && menus.is_empty() {
            return Self::layout_columns(&displays, body, h_gap);
        }
        self.layout_rows(visible, body, v_gap)
    }

    #[expect(clippy::cast_possible_truncation, reason = "heights bounded by u16")]
    fn distribute_heights(&self, indices: &[usize], total_h: u16, v_gap: u16) -> Vec<u16> {
        let n = indices.len();
        if n == 0 {
            return Vec::new();
        }
        let total_gaps = v_gap.saturating_mul((n.saturating_sub(1)) as u16);
        let avail = total_h.saturating_sub(total_gaps);

        let mut heights = vec![0_u16; n];
        let mut used = 0_u16;
        for (i, &sec_idx) in indices.iter().enumerate() {
            let h = self.sections[sec_idx]
                .min_height()
                .max(3)
                .min(avail.saturating_sub(used));
            heights[i] = h;
            used = used.saturating_add(h);
        }

        let mut rem = avail.saturating_sub(used);
        if rem > 0 {
            for (i, &sec_idx) in indices.iter().enumerate() {
                if rem == 0 {
                    break;
                }
                let want = self.sections[sec_idx]
                    .preferred_height()
                    .saturating_sub(heights[i]);
                if want > 0 {
                    let give = want.min(rem);
                    heights[i] += give;
                    rem -= give;
                }
            }
        }
        if rem > 0 {
            let per = rem / (n as u16);
            let extra = rem % (n as u16);
            for (i, h) in heights.iter_mut().enumerate() {
                *h += per + u16::from((i as u16) < extra);
            }
        }
        heights
    }
}
