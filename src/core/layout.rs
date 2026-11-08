//! Declarative spatial constraint solver (`Layout` and `Constraint`).
//!
//! Incorporates the iterative `min_size` locking and fractional remainder distribution
//! algorithm from `textual/_layout_resolve.py` so child [`Rect`]s never overflow or
//! underflow the parent [`Rect`].

use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
use crate::core::rect::Rect;

/// Axis along which a [`Layout`] splits a [`Rect`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::LayoutDirection;
/// assert_eq!(LayoutDirection::default(), LayoutDirection::Vertical);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LayoutDirection {
    /// Stack child rectangles from top to bottom.
    #[default]
    Vertical,
    /// Place child rectangles side by side from left to right.
    Horizontal,
}

/// How a [`Layout`] distributes space left over after every slot is sized
/// (only relevant when no flexible `Fill` / `Min` / `Max` slot absorbs it).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Constraint, Flex, Layout, Rect};
///
/// let area = Rect::from_xywh(0, 0, 20, 1);
/// let centered = Layout::horizontal([Constraint::Fixed(4)]).with_flex(Flex::Center).split(area);
/// assert_eq!(centered[0].left().get(), 8);
/// let ends = Layout::horizontal([Constraint::Fixed(4), Constraint::Fixed(4)])
///     .with_flex(Flex::SpaceBetween)
///     .split(area);
/// assert_eq!(ends[1].right().get(), 20);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Flex {
    /// Pack slots at the start; leftover space trails (default).
    #[default]
    Start,
    /// Pack slots at the end; leftover space leads.
    End,
    /// Center the packed slots.
    Center,
    /// Equal space between slots; none at the edges.
    SpaceBetween,
    /// Equal space around every slot (half-size at the edges).
    SpaceAround,
    /// Equal space between slots and at both edges.
    SpaceEvenly,
}

impl Flex {
    /// Returns `(lead, extra_between)` cells for `leftover` spare cells among `count` slots.
    fn distribute(self, leftover: u16, count: usize) -> (u16, u16) {
        let spare_cells = u32::from(leftover);
        let slots = u32::try_from(count).unwrap_or(u32::MAX);
        let (lead, between) = match self {
            Self::End => (spare_cells, 0),
            Self::Center => (spare_cells / 2, 0),
            Self::SpaceBetween if slots > 1 => (0, spare_cells / (slots - 1)),
            Self::SpaceAround if slots > 0 => {
                let margin = spare_cells / slots.saturating_mul(2);
                (margin, margin.saturating_mul(2))
            }
            Self::SpaceEvenly => {
                let even_gap = spare_cells / slots.saturating_add(1);
                (even_gap, even_gap)
            }
            // `Start`, and degenerate cases (no slots) keep everything packed at the start.
            _ => (0, 0),
        };
        (
            u16::try_from(lead).unwrap_or(u16::MAX),
            u16::try_from(between).unwrap_or(u16::MAX),
        )
    }
}

/// Spatial sizing rule for a single child slot in a [`Layout`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::Constraint;
///
/// let c = Constraint::Percentage(50);
/// assert_eq!(c, Constraint::Percentage(50));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Constraint {
    /// Exact number of cells (clamped to remaining available space).
    Fixed(u16),
    /// Percentage (`0..=100`) of the parent dimension.
    Percentage(u16),
    /// A `numerator / denominator` fraction of the available space (`Ratio(1, 3)` is a third).
    /// A zero denominator yields an empty slot.
    Ratio(u32, u32),
    /// Minimum number of cells; participates in flexible space distribution with weight `1`.
    Min(u16),
    /// Maximum number of cells; takes up to `max` cells from flexible space.
    Max(u16),
    /// Proportional share of remaining flexible space (`Fill(weight)`).
    Fill(u16),
    /// Proportional share of remaining flexible space with a guaranteed minimum floor (`textual` edge).
    FillMin {
        /// Relative weight (`>= 1`).
        weight: u16,
        /// Minimum size floor before proportional distribution.
        min_size: u16,
    },
}

impl Constraint {
    /// An exact size in cells (alias of [`Constraint::Fixed`], familiar from other TUI libraries).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::Constraint;
    /// assert_eq!(Constraint::length(5), Constraint::Fixed(5));
    /// ```
    #[must_use]
    #[inline]
    pub const fn length(cells: u16) -> Self {
        Self::Fixed(cells)
    }
}

impl Default for Constraint {
    fn default() -> Self {
        Self::Fill(1)
    }
}

/// Declarative layout splitter that divides a parent [`Rect`] into child [`Rect`]s.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{ColumnIndex, Constraint, Layout, Rect, RowIndex, TerminalHeight, TerminalWidth};
///
/// let area = Rect::new(ColumnIndex(0), RowIndex(0), TerminalWidth(100), TerminalHeight(20));
/// let chunks = Layout::horizontal([Constraint::Percentage(30), Constraint::Fill(1)]).split(area);
/// assert_eq!(chunks.len(), 2);
/// assert_eq!(chunks[0].width(), TerminalWidth(30));
/// assert_eq!(chunks[1].width(), TerminalWidth(70));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    direction: LayoutDirection,
    constraints: Vec<Constraint>,
    gap: u16,
    flex: Flex,
}

impl Layout {
    /// Creates a vertical layout (top-to-bottom) with the given constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Constraint, Layout, Rect};
    ///
    /// let chunks = Layout::vertical([Constraint::Fixed(3), Constraint::Fill(1)])
    ///     .split(Rect::from_xywh(0, 0, 80, 24));
    /// assert_eq!(chunks[0].height().as_u16(), 3);
    /// assert_eq!(chunks[1].height().as_u16(), 21);
    /// ```
    #[must_use]
    pub fn vertical(constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self {
            direction: LayoutDirection::Vertical,
            constraints: constraints.into_iter().collect(),
            gap: 0,
            flex: Flex::Start,
        }
    }

    /// Creates a horizontal layout (left-to-right) with the given constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Constraint, Layout, Rect};
    ///
    /// let chunks = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
    ///     .with_gap(2)
    ///     .split(Rect::from_xywh(0, 0, 82, 10));
    /// assert_eq!(chunks[0].width().as_u16(), 40);
    /// assert_eq!(chunks[1].width().as_u16(), 40);
    /// ```
    #[must_use]
    pub fn horizontal(constraints: impl IntoIterator<Item = Constraint>) -> Self {
        Self {
            direction: LayoutDirection::Horizontal,
            constraints: constraints.into_iter().collect(),
            gap: 0,
            flex: Flex::Start,
        }
    }

    /// Sets the inter-panel gap in cells between adjacent child rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Constraint, Layout};
    /// let layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).with_gap(2);
    /// assert_eq!(layout.gap(), 2);
    /// ```
    #[must_use]
    pub const fn with_gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Sets how spare space is distributed when no flexible slot absorbs it (default
    /// [`Flex::Start`]).
    #[must_use]
    pub const fn with_flex(mut self, flex: Flex) -> Self {
        self.flex = flex;
        self
    }

    /// Returns the configured [`Flex`] mode.
    #[must_use]
    pub const fn flex(&self) -> Flex {
        self.flex
    }

    /// Returns the configured inter-panel gap.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::Layout;
    /// assert_eq!(Layout::vertical([]).gap(), 0);
    /// ```
    #[must_use]
    pub const fn gap(&self) -> u16 {
        self.gap
    }

    /// Splits `area` into a `Vec<Rect>` matching the configured constraints.
    ///
    /// Guarantees that every returned [`Rect`] is contained within `area` and never panics
    /// across `0..=u16::MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Constraint, Layout, Rect};
    ///
    /// let area = Rect::from_xywh(0, 0, 0, 0);
    /// let chunks = Layout::horizontal([Constraint::Fixed(10), Constraint::Fill(1)]).split(area);
    /// assert_eq!(chunks.len(), 2);
    /// assert!(chunks[0].is_empty());
    /// ```
    #[must_use]
    pub fn split(&self, area: Rect) -> Vec<Rect> {
        let count = self.constraints.len();
        if count == 0 {
            return Vec::new();
        }
        let mut resolved = vec![None; count];
        let mut sizes = vec![0_u16; count];
        let mut out = vec![Rect::default(); count];
        self.solve_into(area, &mut resolved, &mut sizes);
        self.place_into(area, &sizes, &mut out);
        out
    }

    /// Splits `area` into `out` without allocating (up to [`Layout::MAX_STACK_SLOTS`] slots) and
    /// returns how many rectangles were written (`min(out.len(), constraint count)`).
    ///
    /// Intended for hot render paths; build the [`Layout`] once and reuse it.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Constraint, Layout, Rect};
    ///
    /// let layout = Layout::horizontal([Constraint::Fixed(10), Constraint::Fill(1)]);
    /// let mut cells = [Rect::default(); 2];
    /// assert_eq!(layout.split_into(Rect::from_xywh(0, 0, 40, 1), &mut cells), 2);
    /// assert_eq!(cells[1].width().get(), 30);
    /// ```
    pub fn split_into(&self, area: Rect, out: &mut [Rect]) -> usize {
        let count = self.constraints.len();
        if count == 0 || out.is_empty() {
            return 0;
        }
        if count > Self::MAX_STACK_SLOTS {
            let rects = self.split(area);
            let written = rects.len().min(out.len());
            out[..written].copy_from_slice(&rects[..written]);
            return written;
        }
        let mut resolved = [None; Self::MAX_STACK_SLOTS];
        let mut sizes = [0_u16; Self::MAX_STACK_SLOTS];
        let mut rects = [Rect::default(); Self::MAX_STACK_SLOTS];
        self.solve_into(area, &mut resolved[..count], &mut sizes[..count]);
        self.place_into(area, &sizes[..count], &mut rects[..count]);
        let written = count.min(out.len());
        out[..written].copy_from_slice(&rects[..written]);
        written
    }

    /// Splits `area` into exactly `N` rectangles, destructurable without indexing.
    ///
    /// Missing slots (fewer constraints than `N`) are empty rectangles; extra constraints
    /// are ignored. Never panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Constraint, Layout, Rect};
    ///
    /// let [header, body, footer] = Layout::vertical([
    ///     Constraint::Fixed(1),
    ///     Constraint::Fill(1),
    ///     Constraint::Fixed(1),
    /// ])
    /// .areas(Rect::from_xywh(0, 0, 80, 24));
    /// assert_eq!((header.height().get(), body.height().get(), footer.height().get()), (1, 22, 1));
    /// ```
    #[must_use]
    pub fn areas<const N: usize>(&self, area: Rect) -> [Rect; N] {
        let mut out = [Rect::default(); N];
        let _ = self.split_into(area, &mut out);
        out
    }

    /// Largest slot count handled without heap allocation by [`Layout::split_into`].
    pub const MAX_STACK_SLOTS: usize = 64;

    fn usable_span(&self, area: Rect) -> u16 {
        let total_span = match self.direction {
            LayoutDirection::Horizontal => area.width().as_u16(),
            LayoutDirection::Vertical => area.height().as_u16(),
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped to u16::MAX before cast"
        )]
        let num_gaps = self
            .constraints
            .len()
            .saturating_sub(1)
            .min(usize::from(u16::MAX)) as u16;
        total_span.saturating_sub(self.gap.saturating_mul(num_gaps))
    }

    fn solve_into(&self, area: Rect, resolved: &mut [Option<u16>], sizes: &mut [u16]) {
        resolve_constraints(&self.constraints, self.usable_span(area), resolved);
        for (size, slot) in sizes.iter_mut().zip(resolved.iter()) {
            *size = slot.unwrap_or(0);
        }
    }

    fn place_into(&self, area: Rect, sizes: &[u16], out: &mut [Rect]) {
        let count = sizes.len();
        let used: u32 = sizes.iter().map(|&size| u32::from(size)).sum();
        let leftover = u32::from(self.usable_span(area)).saturating_sub(used);
        let leftover = u16::try_from(leftover).unwrap_or(u16::MAX);
        let (lead, extra_between) = self.flex.distribute(leftover, count);

        let (start_edge, max_edge) = match self.direction {
            LayoutDirection::Horizontal => (area.left().as_u16(), area.right().as_u16()),
            LayoutDirection::Vertical => (area.top().as_u16(), area.bottom().as_u16()),
        };
        let mut cursor = start_edge.saturating_add(lead).min(max_edge);

        for (i, (&size, slot)) in sizes.iter().zip(out.iter_mut()).enumerate() {
            let clamped_cursor = cursor.min(max_edge);
            let clamped_size = size.min(max_edge.saturating_sub(clamped_cursor));
            *slot = match self.direction {
                LayoutDirection::Horizontal => Rect::new(
                    ColumnIndex(clamped_cursor),
                    area.top(),
                    TerminalWidth(clamped_size),
                    area.height(),
                ),
                LayoutDirection::Vertical => Rect::new(
                    area.left(),
                    RowIndex(clamped_cursor),
                    area.width(),
                    TerminalHeight(clamped_size),
                ),
            };
            cursor = clamped_cursor.saturating_add(clamped_size);
            if i + 1 < count {
                cursor = cursor
                    .saturating_add(self.gap)
                    .saturating_add(extra_between)
                    .min(max_edge);
            }
        }
    }
}

/// Divides `total` into `n` integer widths separated by `(n - 1) * gap` cells (`de_page.go` `splitWidthsN`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::split_equal_spans;
///
/// assert_eq!(split_equal_spans(100, 2, 2), vec![49, 49]);
/// assert_eq!(split_equal_spans(101, 2, 2), vec![50, 49]);
/// ```
#[must_use]
pub fn split_equal_spans(total: u16, gap: u16, count: usize) -> Vec<u16> {
    if count == 0 {
        return Vec::new();
    }
    #[expect(clippy::cast_possible_truncation, reason = "clamped to u16::MAX")]
    let gaps = count.saturating_sub(1).min(usize::from(u16::MAX)) as u16;
    let avail = total.saturating_sub(gap.saturating_mul(gaps)) as usize;
    let base = avail / count;
    let rem = avail % count;

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let extra = usize::from(i < rem);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "base + extra <= avail <= u16::MAX"
        )]
        out.push((base + extra) as u16);
    }
    out
}

/// Resolves a slice of [`Constraint`]s within `usable_span` using Textual's iterative
/// min-size locking and exact remainder carrying.
#[expect(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    reason = "shares are bounded by usable_span (u16); single cohesive two-pass solver"
)]
fn resolve_constraints(constraints: &[Constraint], usable_span: u16, resolved: &mut [Option<u16>]) {
    let mut remaining = usable_span;

    // Pass 1: Allocate deterministic Fixed and Percentage constraints
    for (i, &c) in constraints.iter().enumerate() {
        match c {
            Constraint::Fixed(size) => {
                let alloc = size.min(remaining);
                resolved[i] = Some(alloc);
                remaining = remaining.saturating_sub(alloc);
            }
            Constraint::Percentage(pct) => {
                let clamped_pct = u32::from(pct.min(100));
                let wanted = ((u32::from(usable_span) * clamped_pct) / 100) as u16;
                let alloc = wanted.min(remaining);
                resolved[i] = Some(alloc);
                remaining = remaining.saturating_sub(alloc);
            }
            Constraint::Ratio(numerator, denominator) => {
                let wanted = if denominator == 0 {
                    0
                } else {
                    (u64::from(usable_span) * u64::from(numerator) / u64::from(denominator))
                        .min(u64::from(u16::MAX)) as u16
                };
                let alloc = wanted.min(remaining);
                resolved[i] = Some(alloc);
                remaining = remaining.saturating_sub(alloc);
            }
            Constraint::Min(_)
            | Constraint::Max(_)
            | Constraint::Fill(_)
            | Constraint::FillMin { .. } => {}
        }
    }

    // Pass 2: Iteratively resolve flexible constraints (Min, Max, Fill, FillMin)
    loop {
        let mut total_weight = 0u32;
        let mut flex_count = 0usize;

        for (i, &c) in constraints.iter().enumerate() {
            if resolved[i].is_some() {
                continue;
            }
            let w = match c {
                Constraint::Fill(weight) | Constraint::FillMin { weight, .. } => {
                    u32::from(weight.max(1))
                }
                Constraint::Min(_) | Constraint::Max(_) => 1,
                Constraint::Fixed(_) | Constraint::Percentage(_) | Constraint::Ratio(..) => 0,
            };
            total_weight = total_weight.saturating_add(w);
            flex_count = flex_count.saturating_add(1);
        }

        if flex_count == 0 || total_weight == 0 {
            break;
        }

        if remaining == 0 {
            for item in resolved.iter_mut() {
                if item.is_none() {
                    *item = Some(0);
                }
            }
            break;
        }

        // Check if any Min / FillMin / Max constraint violates its floor/ceiling at proportional share
        let mut locked_any = false;
        for (i, &c) in constraints.iter().enumerate() {
            if resolved[i].is_some() {
                continue;
            }
            match c {
                Constraint::Min(min_sz)
                | Constraint::FillMin {
                    min_size: min_sz, ..
                } => {
                    let w = match c {
                        Constraint::FillMin { weight, .. } => u32::from(weight.max(1)),
                        _ => 1,
                    };
                    let share = ((u32::from(remaining) * w) / total_weight) as u16;
                    if share < min_sz {
                        let alloc = min_sz.min(remaining);
                        resolved[i] = Some(alloc);
                        remaining = remaining.saturating_sub(alloc);
                        locked_any = true;
                        break;
                    }
                }
                Constraint::Max(max_sz) => {
                    let share = (u32::from(remaining) / total_weight) as u16;
                    if share > max_sz {
                        let alloc = max_sz.min(remaining);
                        resolved[i] = Some(alloc);
                        remaining = remaining.saturating_sub(alloc);
                        locked_any = true;
                        break;
                    }
                }
                _ => {}
            }
        }

        if locked_any {
            continue;
        }

        // Distribute remaining flexible space with exact remainder carrying
        let mut rem_carry = 0u32;
        let pool = u32::from(remaining);
        for (i, &c) in constraints.iter().enumerate() {
            if resolved[i].is_some() {
                continue;
            }
            let w = match c {
                Constraint::Fill(weight) | Constraint::FillMin { weight, .. } => {
                    u32::from(weight.max(1))
                }
                _ => 1,
            };
            let numerator = pool.saturating_mul(w).saturating_add(rem_carry);
            let share = (numerator / total_weight) as u16;
            rem_carry = numerator % total_weight;
            let alloc = share.min(remaining);
            resolved[i] = Some(alloc);
            remaining = remaining.saturating_sub(alloc);
        }
        break;
    }
}
