//! Strongly-typed domain primitives for terminal grid coordinates and dimensions.
//!
//! Using distinct newtypes (`ColumnIndex`, `RowIndex`, `TerminalWidth`, `TerminalHeight`)
//! prevents accidental transposition of X/Y axes or width/height dimensions at compile time.

use core::fmt;

/// Zero-based horizontal column coordinate on a terminal screen or buffer (`0..=u16::MAX`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::ColumnIndex;
///
/// let col = ColumnIndex::new(5).saturating_add(10);
/// assert_eq!(col.as_u16(), 15);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColumnIndex(pub u16);

impl ColumnIndex {
    /// Origin column (`0`).
    pub const ZERO: Self = Self(0);

    /// Creates a new `ColumnIndex` from a raw `u16` coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::ColumnIndex;
    ///
    /// let col = ColumnIndex::new(42);
    /// assert_eq!(col.as_u16(), 42);
    /// ```
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the underlying `u16` column index.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::ColumnIndex;
    ///
    /// assert_eq!(ColumnIndex::new(7).as_u16(), 7);
    /// ```
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Alias for [`Self::as_u16`].
    #[must_use]
    #[inline]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the column index as `usize` for slice indexing.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::ColumnIndex;
    ///
    /// assert_eq!(ColumnIndex::new(7).as_usize(), 7usize);
    /// ```
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Adds an offset using saturating arithmetic so overflow never occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::ColumnIndex;
    ///
    /// assert_eq!(ColumnIndex::new(u16::MAX).saturating_add(5), ColumnIndex::new(u16::MAX));
    /// ```
    #[must_use]
    pub const fn saturating_add(self, offset: u16) -> Self {
        Self(self.0.saturating_add(offset))
    }

    /// Subtracts an offset using saturating arithmetic so underflow never occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::ColumnIndex;
    ///
    /// assert_eq!(ColumnIndex::new(2).saturating_sub(10), ColumnIndex::ZERO);
    /// ```
    #[must_use]
    pub const fn saturating_sub(self, offset: u16) -> Self {
        Self(self.0.saturating_sub(offset))
    }
}

impl fmt::Display for ColumnIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for ColumnIndex {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

/// Zero-based vertical row coordinate on a terminal screen or buffer (`0..=u16::MAX`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::RowIndex;
///
/// let row = RowIndex::new(3).saturating_add(4);
/// assert_eq!(row.as_u16(), 7);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RowIndex(pub u16);

impl RowIndex {
    /// Origin row (`0`).
    pub const ZERO: Self = Self(0);

    /// Creates a new `RowIndex` from a raw `u16` coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::RowIndex;
    ///
    /// assert_eq!(RowIndex::new(12).as_u16(), 12);
    /// ```
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the underlying `u16` row index.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::RowIndex;
    ///
    /// assert_eq!(RowIndex::new(4).as_u16(), 4);
    /// ```
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Alias for [`Self::as_u16`].
    #[must_use]
    #[inline]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the row index as `usize` for slice indexing.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::RowIndex;
    ///
    /// assert_eq!(RowIndex::new(4).as_usize(), 4usize);
    /// ```
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Adds an offset using saturating arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::RowIndex;
    ///
    /// assert_eq!(RowIndex::new(5).saturating_add(3), RowIndex::new(8));
    /// ```
    #[must_use]
    pub const fn saturating_add(self, offset: u16) -> Self {
        Self(self.0.saturating_add(offset))
    }

    /// Subtracts an offset using saturating arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::RowIndex;
    ///
    /// assert_eq!(RowIndex::new(5).saturating_sub(9), RowIndex::ZERO);
    /// ```
    #[must_use]
    pub const fn saturating_sub(self, offset: u16) -> Self {
        Self(self.0.saturating_sub(offset))
    }
}

impl fmt::Display for RowIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for RowIndex {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

/// Horizontal span in terminal character cells (`0..=u16::MAX`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::TerminalWidth;
///
/// let width = TerminalWidth::new(80);
/// assert!(!width.is_zero());
/// assert_eq!(width.saturating_sub(2).as_u16(), 78);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TerminalWidth(pub u16);

impl TerminalWidth {
    /// Zero width (`0`).
    pub const ZERO: Self = Self(0);

    /// Creates a new `TerminalWidth` from a cell count.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    ///
    /// assert_eq!(TerminalWidth::new(120).as_u16(), 120);
    /// ```
    #[must_use]
    pub const fn new(cells: u16) -> Self {
        Self(cells)
    }

    /// Returns true if the width is zero cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    ///
    /// assert!(TerminalWidth::ZERO.is_zero());
    /// assert!(!TerminalWidth::new(1).is_zero());
    /// ```
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns the raw `u16` cell count.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    ///
    /// assert_eq!(TerminalWidth::new(40).as_u16(), 40);
    /// ```
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Alias for [`Self::as_u16`].
    #[must_use]
    #[inline]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the cell count as `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    ///
    /// assert_eq!(TerminalWidth::new(40).as_usize(), 40usize);
    /// ```
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Adds cells using saturating arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    ///
    /// assert_eq!(TerminalWidth::new(10).saturating_add(5).as_u16(), 15);
    /// ```
    #[must_use]
    pub const fn saturating_add(self, cells: u16) -> Self {
        Self(self.0.saturating_add(cells))
    }

    /// Subtracts cells using saturating arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    ///
    /// assert_eq!(TerminalWidth::new(1).saturating_sub(4), TerminalWidth::ZERO);
    /// ```
    #[must_use]
    pub const fn saturating_sub(self, cells: u16) -> Self {
        Self(self.0.saturating_sub(cells))
    }
}

impl fmt::Display for TerminalWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for TerminalWidth {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

/// Vertical span in terminal character rows (`0..=u16::MAX`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::TerminalHeight;
///
/// let height = TerminalHeight::new(24);
/// assert!(!height.is_zero());
/// assert_eq!(height.saturating_sub(2).as_u16(), 22);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TerminalHeight(pub u16);

impl TerminalHeight {
    /// Zero height (`0`).
    pub const ZERO: Self = Self(0);

    /// Creates a new `TerminalHeight` from a row count.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalHeight;
    ///
    /// assert_eq!(TerminalHeight::new(30).as_u16(), 30);
    /// ```
    #[must_use]
    pub const fn new(rows: u16) -> Self {
        Self(rows)
    }

    /// Returns true if the height is zero rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalHeight;
    ///
    /// assert!(TerminalHeight::ZERO.is_zero());
    /// ```
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns the raw `u16` row count.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalHeight;
    ///
    /// assert_eq!(TerminalHeight::new(24).as_u16(), 24);
    /// ```
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Alias for [`Self::as_u16`].
    #[must_use]
    #[inline]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Returns the row count as `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalHeight;
    ///
    /// assert_eq!(TerminalHeight::new(24).as_usize(), 24usize);
    /// ```
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Adds rows using saturating arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalHeight;
    ///
    /// assert_eq!(TerminalHeight::new(10).saturating_add(2).as_u16(), 12);
    /// ```
    #[must_use]
    pub const fn saturating_add(self, rows: u16) -> Self {
        Self(self.0.saturating_add(rows))
    }

    /// Subtracts rows using saturating arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalHeight;
    ///
    /// assert_eq!(TerminalHeight::new(2).saturating_sub(5), TerminalHeight::ZERO);
    /// ```
    #[must_use]
    pub const fn saturating_sub(self, rows: u16) -> Self {
        Self(self.0.saturating_sub(rows))
    }
}

impl fmt::Display for TerminalHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for TerminalHeight {
    fn from(value: u16) -> Self {
        Self(value)
    }
}
