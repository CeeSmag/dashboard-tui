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
/// use tui_rs::core::LayoutDirection;
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

/// Spatial sizing rule for a single child slot in a [`Layout`].
///
/// # Examples
///
/// ```
/// use tui_rs::core::Constraint;
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
/// use tui_rs::core::{ColumnIndex, Constraint, Layout, Rect, RowIndex, TerminalHeight, TerminalWidth};
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
}

impl Layout {
    /// Creates a vertical layout (top-to-bottom) with the given constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Constraint, Layout, Rect};
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
        }
    }

    /// Creates a horizontal layout (left-to-right) with the given constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Constraint, Layout, Rect};
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
        }
    }

    /// Sets the inter-panel gap in cells between adjacent child rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Constraint, Layout};
    /// let layout = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).with_gap(2);
    /// assert_eq!(layout.gap(), 2);
    /// ```
    #[must_use]
    pub const fn with_gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Returns the configured inter-panel gap.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Layout;
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
    /// use tui_rs::core::{Constraint, Layout, Rect};
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

        let total_span = match self.direction {
            LayoutDirection::Horizontal => area.width().as_u16(),
            LayoutDirection::Vertical => area.height().as_u16(),
        };

        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped to u16::MAX before cast"
        )]
        let num_gaps = count.saturating_sub(1).min(usize::from(u16::MAX)) as u16;
        let total_gap = self.gap.saturating_mul(num_gaps);
        let usable_span = total_span.saturating_sub(total_gap);

        let sizes = resolve_constraints(&self.constraints, usable_span);

        let mut out = Vec::with_capacity(count);
        let mut cursor = match self.direction {
            LayoutDirection::Horizontal => area.left().as_u16(),
            LayoutDirection::Vertical => area.top().as_u16(),
        };
        let max_edge = match self.direction {
            LayoutDirection::Horizontal => area.right().as_u16(),
            LayoutDirection::Vertical => area.bottom().as_u16(),
        };

        for (i, &size) in sizes.iter().enumerate() {
            let clamped_cursor = cursor.min(max_edge);
            let clamped_size = size.min(max_edge.saturating_sub(clamped_cursor));
            let rect = match self.direction {
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
            out.push(rect);
            cursor = clamped_cursor.saturating_add(clamped_size);
            if i + 1 < count {
                cursor = cursor.saturating_add(self.gap).min(max_edge);
            }
        }

        out
    }
}

/// Divides `total` into `n` integer widths separated by `(n - 1) * gap` cells (`de_page.go` `splitWidthsN`).
///
/// # Examples
///
/// ```
/// use tui_rs::core::split_equal_spans;
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
fn resolve_constraints(constraints: &[Constraint], usable_span: u16) -> Vec<u16> {
    let n = constraints.len();
    let mut resolved: Vec<Option<u16>> = vec![None; n];
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
                Constraint::Fixed(_) | Constraint::Percentage(_) => 0,
            };
            total_weight = total_weight.saturating_add(w);
            flex_count = flex_count.saturating_add(1);
        }

        if flex_count == 0 || total_weight == 0 {
            break;
        }

        if remaining == 0 {
            for item in &mut resolved {
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

    resolved.into_iter().map(|opt| opt.unwrap_or(0)).collect()
}
