//! Priority-weighted header segment and footer key-hint strip fitters (`PrioritySegment`, `KeyHint`).
//!
//! Direct zero-allocation Rust equivalent of `btFitSegments` and `btFitHints` from `de_layout.go`:
//! when terminal width shrinks, the lowest-priority segments or key hints are dropped one by one
//! until the strip fits within the column budget.

use std::borrow::Cow;

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::display_width;
use crate::core::style::Style;

/// A single prioritized segment in the top dashboard header strip (`btSegment` in `de_layout.go`).
///
/// Segment `0` in a slice is never dropped so the program title/identity always remains visible.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{PrioritySegment, Style};
///
/// let seg = PrioritySegment::new("VaultDrive v2.2.0", Style::DEFAULT, 100);
/// assert_eq!(seg.priority(), 100);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrioritySegment<'a> {
    text: Cow<'a, str>,
    style: Style,
    priority: i16,
}

impl<'a> PrioritySegment<'a> {
    /// Creates a new prioritized header segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{PrioritySegment, Style};
    /// let s = PrioritySegment::new("● MOUNTED", Style::DEFAULT, 90);
    /// assert_eq!(s.as_str(), "● MOUNTED");
    /// ```
    #[must_use]
    pub fn new(text: impl Into<Cow<'a, str>>, style: Style, priority: i16) -> Self {
        Self {
            text: text.into(),
            style,
            priority,
        }
    }

    /// Returns the text slice of this segment (`C-CONV` `as_str`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{PrioritySegment, Style};
    /// assert_eq!(PrioritySegment::new("A", Style::DEFAULT, 1).as_str(), "A");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns the visual [`Style`] of this segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{PrioritySegment, Style};
    /// assert_eq!(PrioritySegment::new("A", Style::DEFAULT, 1).style(), Style::DEFAULT);
    /// ```
    #[must_use]
    pub const fn style(&self) -> Style {
        self.style
    }

    /// Returns the retention priority (higher values survive longer on narrow screens).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{PrioritySegment, Style};
    /// assert_eq!(PrioritySegment::new("A", Style::DEFAULT, 50).priority(), 50);
    /// ```
    #[must_use]
    pub const fn priority(&self) -> i16 {
        self.priority
    }

    /// Returns the visual column width of this segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{PrioritySegment, Style, TerminalWidth};
    /// assert_eq!(PrioritySegment::new("1234", Style::DEFAULT, 1).width(), TerminalWidth(4));
    /// ```
    #[must_use]
    pub fn width(&self) -> TerminalWidth {
        display_width(&self.text)
    }
}

/// Computes which indices in `segments` survive within `budget` columns when joined by `" │ "` (3 cols).
///
/// Matches `btFitSegments` in `de_layout.go`: index `0` is never dropped.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{fit_segment_indices, PrioritySegment, Style, TerminalWidth};
///
/// let segs = [
///     PrioritySegment::new("VaultDrive", Style::DEFAULT, 100),
///     PrioritySegment::new("LowPrio", Style::DEFAULT, 10),
///     PrioritySegment::new("HighPrio", Style::DEFAULT, 90),
/// ];
/// // "VaultDrive │ HighPrio" = 10 + 3 + 8 = 21 cols
/// let kept = fit_segment_indices(&segs, TerminalWidth(22));
/// assert_eq!(kept, vec![0, 2]);
/// ```
#[must_use]
pub fn fit_segment_indices(segments: &[PrioritySegment<'_>], budget: TerminalWidth) -> Vec<usize> {
    let n = segments.len();
    if n == 0 {
        return Vec::new();
    }
    let mut keep: Vec<bool> = segments.iter().map(|s| !s.as_str().is_empty()).collect();

    loop {
        let mut total_w = 0u16;
        let mut count_kept = 0usize;
        for (i, seg) in segments.iter().enumerate() {
            if keep[i] {
                if count_kept > 0 {
                    total_w = total_w.saturating_add(3); // " │ "
                }
                total_w = total_w.saturating_add(seg.width().as_u16());
                count_kept = count_kept.saturating_add(1);
            }
        }

        if total_w <= budget.as_u16() {
            break;
        }

        // Drop the lowest-priority segment among indices 1..n
        let mut worst_idx: Option<usize> = None;
        for i in 1..n {
            if !keep[i] {
                continue;
            }
            if let Some(w) = worst_idx {
                if segments[i].priority < segments[w].priority {
                    worst_idx = Some(i);
                }
            } else {
                worst_idx = Some(i);
            }
        }

        if let Some(w) = worst_idx {
            keep[w] = false;
        } else {
            break;
        }
    }

    (0..n).filter(|&i| keep[i]).collect()
}

/// Renders the surviving [`PrioritySegment`]s into `buffer` at `(column, row)` within `budget` columns.
///
/// Returns the ending [`ColumnIndex`].
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColumnIndex, PrioritySegment, Rect, render_fitted_segments, RowIndex, Style, TerminalWidth};
///
/// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 20, 1));
/// let segs = [
///     PrioritySegment::new("App", Style::DEFAULT, 100),
///     PrioritySegment::new("OK", Style::DEFAULT, 80),
/// ];
/// let end = render_fitted_segments(&mut buf, ColumnIndex(0), RowIndex(0), TerminalWidth(20), &segs, Style::DEFAULT);
/// assert_eq!(end, ColumnIndex(8));
/// assert_eq!(buf.to_visual_lines(), vec!["App │ OK            "]);
/// ```
pub fn render_fitted_segments(
    buffer: &mut Buffer,
    column: ColumnIndex,
    row: RowIndex,
    budget: TerminalWidth,
    segments: &[PrioritySegment<'_>],
    separator_style: Style,
) -> ColumnIndex {
    let kept = fit_segment_indices(segments, budget);
    let max_col = column.saturating_add(budget.as_u16());
    let mut cur = column;

    for (pos, &idx) in kept.iter().enumerate() {
        if cur >= max_col {
            break;
        }
        if pos > 0 {
            cur = buffer.set_str_clipped(cur, row, " │ ", max_col, separator_style);
        }
        let seg = &segments[idx];
        cur = buffer.set_str_clipped(cur, row, seg.as_str(), max_col, seg.style());
    }
    cur
}

/// A single prioritized keyboard shortcut hint in the footer strip (`btHint` in `de_layout.go`).
///
/// Rendered as `[key] label` where `[key]` uses `key_style` and `label` uses `muted_style`.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{KeyHint, Style};
///
/// let hint = KeyHint::new("q", "Quit", Style::DEFAULT, 100);
/// assert_eq!(hint.key(), "q");
/// assert_eq!(hint.label(), "Quit");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyHint<'a> {
    key: Cow<'a, str>,
    label: Cow<'a, str>,
    key_style: Style,
    priority: i16,
}

impl<'a> KeyHint<'a> {
    /// Creates a new footer key hint (`[key] label`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{KeyHint, Style, TerminalWidth};
    /// let h = KeyHint::new("Tab", "Page", Style::DEFAULT, 80);
    /// // "[Tab] Page" = 5 + 1 + 4 = 10 columns
    /// assert_eq!(h.width(), TerminalWidth(10));
    /// ```
    #[must_use]
    pub fn new(
        key: impl Into<Cow<'a, str>>,
        label: impl Into<Cow<'a, str>>,
        key_style: Style,
        priority: i16,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            key_style,
            priority,
        }
    }

    /// Returns the shortcut key text (`"q"`, `"Tab"`, `"↑↓"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{KeyHint, Style};
    /// assert_eq!(KeyHint::new("m", "Mount", Style::DEFAULT, 1).key(), "m");
    /// ```
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the action label text (`"Quit"`, `"Next Page"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{KeyHint, Style};
    /// assert_eq!(KeyHint::new("m", "Mount", Style::DEFAULT, 1).label(), "Mount");
    /// ```
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the style applied to `[key]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{KeyHint, Style};
    /// assert_eq!(KeyHint::new("m", "Mount", Style::DEFAULT, 1).key_style(), Style::DEFAULT);
    /// ```
    #[must_use]
    pub const fn key_style(&self) -> Style {
        self.key_style
    }

    /// Returns the retention priority of this hint.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{KeyHint, Style};
    /// assert_eq!(KeyHint::new("q", "Quit", Style::DEFAULT, 100).priority(), 100);
    /// ```
    #[must_use]
    pub const fn priority(&self) -> i16 {
        self.priority
    }

    /// Computes the visual column width of `[key] label` (`key_width + 3 + label_width`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{KeyHint, Style, TerminalWidth};
    /// assert_eq!(KeyHint::new("q", "Quit", Style::DEFAULT, 100).width(), TerminalWidth(8));
    /// ```
    #[must_use]
    pub fn width(&self) -> TerminalWidth {
        let kw = display_width(&self.key).as_u16();
        let lw = display_width(&self.label).as_u16();
        TerminalWidth(kw.saturating_add(3).saturating_add(lw))
    }
}

/// Computes which indices in `hints` survive within `budget` columns when separated by a single space.
///
/// Matches `btFitHints` in `de_layout.go`.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{fit_hint_indices, KeyHint, Style, TerminalWidth};
///
/// let hints = [
///     KeyHint::new("m", "Mount", Style::DEFAULT, 40),
///     KeyHint::new("q", "Quit", Style::DEFAULT, 100),
/// ];
/// let kept = fit_hint_indices(&hints, TerminalWidth(10));
/// assert_eq!(kept, vec![1]);
/// ```
#[must_use]
pub fn fit_hint_indices(hints: &[KeyHint<'_>], budget: TerminalWidth) -> Vec<usize> {
    let n = hints.len();
    if n == 0 {
        return Vec::new();
    }
    let mut keep = vec![true; n];

    loop {
        let mut total_w = 0u16;
        let mut count_kept = 0usize;
        for (i, h) in hints.iter().enumerate() {
            if keep[i] {
                if count_kept > 0 {
                    total_w = total_w.saturating_add(1);
                }
                total_w = total_w.saturating_add(h.width().as_u16());
                count_kept = count_kept.saturating_add(1);
            }
        }

        if total_w <= budget.as_u16() {
            break;
        }

        let mut worst_idx: Option<usize> = None;
        for i in 0..n {
            if !keep[i] {
                continue;
            }
            if let Some(w) = worst_idx {
                if hints[i].priority < hints[w].priority {
                    worst_idx = Some(i);
                }
            } else {
                worst_idx = Some(i);
            }
        }

        if let Some(w) = worst_idx {
            keep[w] = false;
        } else {
            break;
        }
    }

    (0..n).filter(|&i| keep[i]).collect()
}

/// Renders the surviving [`KeyHint`]s into `buffer` at `(column, row)` within `budget` columns.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColumnIndex, KeyHint, Rect, render_fitted_hints, RowIndex, Style, TerminalWidth};
///
/// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 18, 1));
/// let hints = [
///     KeyHint::new("m", "Mount", Style::DEFAULT, 50),
///     KeyHint::new("q", "Quit", Style::DEFAULT, 100),
/// ];
/// render_fitted_hints(&mut buf, ColumnIndex(0), RowIndex(0), TerminalWidth(18), &hints, Style::DEFAULT);
/// assert_eq!(buf.to_visual_lines(), vec!["[m] Mount [q] Quit"]);
/// ```
pub fn render_fitted_hints(
    buffer: &mut Buffer,
    column: ColumnIndex,
    row: RowIndex,
    budget: TerminalWidth,
    hints: &[KeyHint<'_>],
    label_style: Style,
) -> ColumnIndex {
    let kept = fit_hint_indices(hints, budget);
    let max_col = column.saturating_add(budget.as_u16());
    let mut cur = column;

    for (pos, &idx) in kept.iter().enumerate() {
        if cur >= max_col {
            break;
        }
        if pos > 0 {
            cur = buffer.set_str_clipped(cur, row, " ", max_col, label_style);
        }
        let h = &hints[idx];
        cur = buffer.set_str_clipped(cur, row, "[", max_col, h.key_style());
        cur = buffer.set_str_clipped(cur, row, h.key(), max_col, h.key_style());
        cur = buffer.set_str_clipped(cur, row, "]", max_col, h.key_style());
        cur = buffer.set_str_clipped(cur, row, " ", max_col, label_style);
        cur = buffer.set_str_clipped(cur, row, h.label(), max_col, label_style);
    }
    cur
}
