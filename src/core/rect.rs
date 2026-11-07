//! Axis-aligned terminal rectangles (`Rect`) and 4-directional spacing (`Spacing4D`).
//!
//! All geometric calculations use saturating `u16` arithmetic so zero-sized or
//! `u16::MAX`-sized viewports never overflow or underflow.

pub use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};

/// Four-directional padding or margin in terminal cells (`top`, `right`, `bottom`, `left`).
///
/// Modeled after CSS/Textual spacing primitives to power the 4-layer dashboard spacing system.
///
/// # Examples
///
/// ```
/// use tui_rs::core::Spacing4D;
///
/// let uniform = Spacing4D::uniform(2);
/// assert_eq!(uniform.horizontal_total(), 4);
/// assert_eq!(uniform.vertical_total(), 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Spacing4D {
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

impl Spacing4D {
    /// Zero spacing on all four sides.
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// Creates a `Spacing4D` with explicit top, right, bottom, and left cell counts.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    ///
    /// let pad = Spacing4D::new(1, 2, 1, 2);
    /// assert_eq!(pad.top(), 1);
    /// assert_eq!(pad.right(), 2);
    /// ```
    #[must_use]
    pub const fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates uniform spacing on all four sides.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    ///
    /// let pad = Spacing4D::uniform(3);
    /// assert_eq!(pad.left(), 3);
    /// assert_eq!(pad.bottom(), 3);
    /// ```
    #[must_use]
    pub const fn uniform(cells: u16) -> Self {
        Self {
            top: cells,
            right: cells,
            bottom: cells,
            left: cells,
        }
    }

    /// Creates symmetric vertical (`top`/`bottom`) and horizontal (`left`/`right`) spacing.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    ///
    /// let pad = Spacing4D::symmetric(1, 3);
    /// assert_eq!(pad.vertical_total(), 2);
    /// assert_eq!(pad.horizontal_total(), 6);
    /// ```
    #[must_use]
    pub const fn symmetric(vertical: u16, horizontal: u16) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Returns the top spacing in rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    /// assert_eq!(Spacing4D::new(2, 0, 0, 0).top(), 2);
    /// ```
    #[must_use]
    pub const fn top(self) -> u16 {
        self.top
    }

    /// Returns the right spacing in columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    /// assert_eq!(Spacing4D::new(0, 4, 0, 0).right(), 4);
    /// ```
    #[must_use]
    pub const fn right(self) -> u16 {
        self.right
    }

    /// Returns the bottom spacing in rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    /// assert_eq!(Spacing4D::new(0, 0, 3, 0).bottom(), 3);
    /// ```
    #[must_use]
    pub const fn bottom(self) -> u16 {
        self.bottom
    }

    /// Returns the left spacing in columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    /// assert_eq!(Spacing4D::new(0, 0, 0, 5).left(), 5);
    /// ```
    #[must_use]
    pub const fn left(self) -> u16 {
        self.left
    }

    /// Returns `left + right` using saturating addition.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    /// assert_eq!(Spacing4D::symmetric(1, 4).horizontal_total(), 8);
    /// ```
    #[must_use]
    pub const fn horizontal_total(self) -> u16 {
        self.left.saturating_add(self.right)
    }

    /// Returns `top + bottom` using saturating addition.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Spacing4D;
    /// assert_eq!(Spacing4D::symmetric(3, 1).vertical_total(), 6);
    /// ```
    #[must_use]
    pub const fn vertical_total(self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

/// An axis-aligned rectangle on a terminal grid.
///
/// `Rect` is the single canonical geometric region type across the entire library.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{ColumnIndex, Rect, RowIndex, Spacing4D, TerminalHeight, TerminalWidth};
///
/// let rect = Rect::new(ColumnIndex::new(0), RowIndex::new(0), TerminalWidth::new(80), TerminalHeight::new(24));
/// let inner = rect.inner(Spacing4D::uniform(1));
/// assert_eq!(inner.left(), ColumnIndex::new(1));
/// assert_eq!(inner.width(), TerminalWidth::new(78));
/// assert_eq!(inner.height(), TerminalHeight::new(22));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    origin_column: ColumnIndex,
    origin_row: RowIndex,
    width: TerminalWidth,
    height: TerminalHeight,
}

impl Rect {
    /// Empty zero-sized rectangle at `(0, 0)`.
    pub const ZERO: Self = Self {
        origin_column: ColumnIndex::ZERO,
        origin_row: RowIndex::ZERO,
        width: TerminalWidth::ZERO,
        height: TerminalHeight::ZERO,
    };

    /// Creates a new `Rect` from typed origin coordinates and dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    ///
    /// let r = Rect::new(ColumnIndex(2), RowIndex(3), TerminalWidth(10), TerminalHeight(5));
    /// assert_eq!(r.right(), ColumnIndex(12));
    /// assert_eq!(r.bottom(), RowIndex(8));
    /// ```
    #[must_use]
    pub const fn new(
        origin_column: ColumnIndex,
        origin_row: RowIndex,
        width: TerminalWidth,
        height: TerminalHeight,
    ) -> Self {
        Self {
            origin_column,
            origin_row,
            width,
            height,
        }
    }

    /// Creates a `Rect` anchored at `(0, 0)` with the given dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    ///
    /// let r = Rect::from_dimensions(TerminalWidth(40), TerminalHeight(10));
    /// assert_eq!(r.left(), ColumnIndex(0));
    /// assert_eq!(r.top(), RowIndex(0));
    /// ```
    #[must_use]
    pub const fn from_dimensions(width: TerminalWidth, height: TerminalHeight) -> Self {
        Self {
            origin_column: ColumnIndex::ZERO,
            origin_row: RowIndex::ZERO,
            width,
            height,
        }
    }

    /// Alias for [`Self::from_dimensions`]: creates a `Rect` at `(0, 0)` with `(width, height)`.
    #[must_use]
    #[inline]
    pub const fn from_size(width: TerminalWidth, height: TerminalHeight) -> Self {
        Self::from_dimensions(width, height)
    }

    /// Leftmost inclusive column coordinate (`x` alias for [`Self::left`]).
    #[must_use]
    #[inline]
    pub const fn x(self) -> ColumnIndex {
        self.origin_column
    }

    /// Topmost inclusive row coordinate (`y` alias for [`Self::top`]).
    #[must_use]
    #[inline]
    pub const fn y(self) -> RowIndex {
        self.origin_row
    }

    /// Convenience constructor from raw `(x, y, width, height)` `u16` values.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Rect;
    ///
    /// let r = Rect::from_xywh(1, 2, 20, 10);
    /// assert_eq!(r.area(), 200);
    /// ```
    #[must_use]
    pub const fn from_xywh(col: u16, row: u16, width: u16, height: u16) -> Self {
        Self {
            origin_column: ColumnIndex(col),
            origin_row: RowIndex(row),
            width: TerminalWidth(width),
            height: TerminalHeight(height),
        }
    }

    /// Returns true if either width or height is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Rect;
    ///
    /// assert!(Rect::from_xywh(0, 0, 0, 10).is_empty());
    /// assert!(!Rect::from_xywh(0, 0, 10, 10).is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }

    /// Returns the total number of cells (`width * height`) as `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Rect;
    ///
    /// let r = Rect::from_xywh(0, 0, 100, 50);
    /// assert_eq!(r.area(), 5000);
    /// ```
    #[must_use]
    pub const fn area(self) -> usize {
        (self.width.0 as usize).saturating_mul(self.height.0 as usize)
    }

    /// Leftmost inclusive column coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{ColumnIndex, Rect};
    /// assert_eq!(Rect::from_xywh(4, 0, 10, 5).left(), ColumnIndex(4));
    /// ```
    #[must_use]
    pub const fn left(self) -> ColumnIndex {
        self.origin_column
    }

    /// Rightmost exclusive column coordinate (`left + width`, saturated at `u16::MAX`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{ColumnIndex, Rect};
    /// assert_eq!(Rect::from_xywh(4, 0, 10, 5).right(), ColumnIndex(14));
    /// ```
    #[must_use]
    pub const fn right(self) -> ColumnIndex {
        self.origin_column.saturating_add(self.width.0)
    }

    /// Topmost inclusive row coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, RowIndex};
    /// assert_eq!(Rect::from_xywh(0, 3, 10, 5).top(), RowIndex(3));
    /// ```
    #[must_use]
    pub const fn top(self) -> RowIndex {
        self.origin_row
    }

    /// Bottommost exclusive row coordinate (`top + height`, saturated at `u16::MAX`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, RowIndex};
    /// assert_eq!(Rect::from_xywh(0, 3, 10, 5).bottom(), RowIndex(8));
    /// ```
    #[must_use]
    pub const fn bottom(self) -> RowIndex {
        self.origin_row.saturating_add(self.height.0)
    }

    /// Width of the rectangle in columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, TerminalWidth};
    /// assert_eq!(Rect::from_xywh(0, 0, 42, 10).width(), TerminalWidth(42));
    /// ```
    #[must_use]
    pub const fn width(self) -> TerminalWidth {
        self.width
    }

    /// Height of the rectangle in rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, TerminalHeight};
    /// assert_eq!(Rect::from_xywh(0, 0, 42, 10).height(), TerminalHeight(10));
    /// ```
    #[must_use]
    pub const fn height(self) -> TerminalHeight {
        self.height
    }

    /// Shrinks the rectangle inward by `spacing` on all four sides using saturating math.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, Spacing4D};
    ///
    /// let small = Rect::from_xywh(0, 0, 2, 2).inner(Spacing4D::uniform(5));
    /// assert!(small.is_empty());
    /// ```
    #[must_use]
    pub const fn inner(self, spacing: Spacing4D) -> Self {
        let h_total = spacing.horizontal_total();
        let v_total = spacing.vertical_total();
        if self.width.0 < h_total || self.height.0 < v_total {
            return Self {
                origin_column: self.origin_column.saturating_add(spacing.left()),
                origin_row: self.origin_row.saturating_add(spacing.top()),
                width: TerminalWidth::ZERO,
                height: TerminalHeight::ZERO,
            };
        }
        Self {
            origin_column: self.origin_column.saturating_add(spacing.left()),
            origin_row: self.origin_row.saturating_add(spacing.top()),
            width: self.width.saturating_sub(h_total),
            height: self.height.saturating_sub(v_total),
        }
    }

    /// Shrinks the rectangle inward by `spacing`, clamping the padding down if needed
    /// so at least `min_width` and `min_height` remain when the outer `Rect` is large enough.
    ///
    /// Implements the small-terminal safety law from `de_page.go` and `de_dashboard.go`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, Spacing4D, TerminalHeight, TerminalWidth};
    ///
    /// let outer = Rect::from_xywh(0, 0, 42, 12);
    /// let clamped = outer.inner_clamped(Spacing4D::uniform(6), TerminalWidth(40), TerminalHeight(10));
    /// assert_eq!(clamped.width(), TerminalWidth(40));
    /// assert_eq!(clamped.height(), TerminalHeight(10));
    /// ```
    #[must_use]
    pub const fn inner_clamped(
        self,
        spacing: Spacing4D,
        min_width: TerminalWidth,
        min_height: TerminalHeight,
    ) -> Self {
        let max_horiz_inset = self.width.0.saturating_sub(min_width.0) / 2;
        let max_vert_inset = self.height.0.saturating_sub(min_height.0) / 2;

        let left = if spacing.left() < max_horiz_inset {
            spacing.left()
        } else {
            max_horiz_inset
        };
        let right = if spacing.right() < max_horiz_inset {
            spacing.right()
        } else {
            max_horiz_inset
        };
        let top = if spacing.top() < max_vert_inset {
            spacing.top()
        } else {
            max_vert_inset
        };
        let bottom = if spacing.bottom() < max_vert_inset {
            spacing.bottom()
        } else {
            max_vert_inset
        };

        self.inner(Spacing4D::new(top, right, bottom, left))
    }

    /// Returns true if `(column, row)` lies strictly within `[left..right)` and `[top..bottom)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{ColumnIndex, Rect, RowIndex};
    ///
    /// let r = Rect::from_xywh(2, 2, 4, 4);
    /// assert!(r.contains(ColumnIndex(2), RowIndex(2)));
    /// assert!(!r.contains(ColumnIndex(6), RowIndex(2)));
    /// ```
    #[must_use]
    pub const fn contains(self, column: ColumnIndex, row: RowIndex) -> bool {
        column.0 >= self.left().0
            && column.0 < self.right().0
            && row.0 >= self.top().0
            && row.0 < self.bottom().0
    }

    /// Computes the overlapping intersection of two rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Rect;
    ///
    /// let a = Rect::from_xywh(0, 0, 10, 10);
    /// let b = Rect::from_xywh(5, 5, 10, 10);
    /// assert_eq!(a.intersection(b), Rect::from_xywh(5, 5, 5, 5));
    /// ```
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        let x1 = if self.left().0 > other.left().0 {
            self.left().0
        } else {
            other.left().0
        };
        let y1 = if self.top().0 > other.top().0 {
            self.top().0
        } else {
            other.top().0
        };
        let x2 = if self.right().0 < other.right().0 {
            self.right().0
        } else {
            other.right().0
        };
        let y2 = if self.bottom().0 < other.bottom().0 {
            self.bottom().0
        } else {
            other.bottom().0
        };
        if x2 <= x1 || y2 <= y1 {
            return Self::ZERO;
        }
        Self::from_xywh(x1, y1, x2.saturating_sub(x1), y2.saturating_sub(y1))
    }

    /// Returns a centered sub-rectangle of at most `(target_width, target_height)` inside `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, TerminalHeight, TerminalWidth};
    ///
    /// let screen = Rect::from_xywh(0, 0, 100, 40);
    /// let modal = screen.centered(TerminalWidth(60), TerminalHeight(20));
    /// assert_eq!(modal, Rect::from_xywh(20, 10, 60, 20));
    /// ```
    #[must_use]
    pub const fn centered(
        self,
        target_width: TerminalWidth,
        target_height: TerminalHeight,
    ) -> Self {
        let clamped_w = if target_width.0 < self.width.0 {
            target_width.0
        } else {
            self.width.0
        };
        let clamped_h = if target_height.0 < self.height.0 {
            target_height.0
        } else {
            self.height.0
        };
        let offset_x = self.width.0.saturating_sub(clamped_w) / 2;
        let offset_y = self.height.0.saturating_sub(clamped_h) / 2;
        Self::from_xywh(
            self.origin_column.0.saturating_add(offset_x),
            self.origin_row.0.saturating_add(offset_y),
            clamped_w,
            clamped_h,
        )
    }
}
