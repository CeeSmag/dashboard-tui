//! Compact terminal grid cell (`Cell`).
//!
//! Stores a single Unicode grapheme cluster inline on the stack via [`compact_str::CompactString`]
//! (zero heap allocation for graphemes up to 24 bytes, including multi-codepoint emojis)
//! alongside its visual [`Style`] and wide-character continuation flag.

use compact_str::CompactString;

use crate::core::style::Style;

/// A single terminal grid cell holding one grapheme cluster and its [`Style`].
///
/// When a 2-column East Asian or emoji grapheme is written to column `c`, the cell at
/// `c + 1` is marked as a continuation cell (`is_wide_continuation() == true`) so the
/// ANSI diff renderer skips emitting a character at `c + 1`.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Cell, Color, Style};
///
/// let mut cell = Cell::new("█", Style::new().fg(Color::Cyan));
/// assert_eq!(cell.symbol(), "█");
/// assert!(!cell.is_wide_continuation());
/// cell.reset();
/// assert_eq!(cell.symbol(), " ");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cell {
    symbol: CompactString,
    style: Style,
    wide_continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Cell {
    /// Blank space cell with default style.
    pub const EMPTY: Self = Self {
        symbol: CompactString::const_new(" "),
        style: Style::DEFAULT,
        wide_continuation: false,
    };

    /// Creates a new `Cell` with the given grapheme symbol and [`Style`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Cell, Style};
    ///
    /// let cell = Cell::new("●", Style::DEFAULT);
    /// assert_eq!(cell.symbol(), "●");
    /// ```
    #[must_use]
    pub fn new(symbol: &str, style: Style) -> Self {
        Self {
            symbol: if symbol.is_empty() {
                CompactString::const_new(" ")
            } else {
                CompactString::new(symbol)
            },
            style,
            wide_continuation: false,
        }
    }

    /// Creates a `Cell` from a single `char` and [`Style`] without heap allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Cell, Style};
    ///
    /// let cell = Cell::from_char('╭', Style::DEFAULT);
    /// assert_eq!(cell.symbol(), "╭");
    /// ```
    #[must_use]
    pub fn from_char(ch: char, style: Style) -> Self {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        Self {
            symbol: CompactString::new(encoded),
            style,
            wide_continuation: false,
        }
    }

    /// Returns the grapheme cluster stored in this cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Cell;
    /// assert_eq!(Cell::EMPTY.symbol(), " ");
    /// ```
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Returns the [`Style`] of this cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Cell, Style};
    /// assert_eq!(Cell::EMPTY.style(), Style::DEFAULT);
    /// ```
    #[must_use]
    pub const fn style(&self) -> Style {
        self.style
    }

    /// Returns true if this cell is the trailing 2nd column of a 2-column wide grapheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Cell;
    /// assert!(!Cell::EMPTY.is_wide_continuation());
    /// ```
    #[must_use]
    pub const fn is_wide_continuation(&self) -> bool {
        self.wide_continuation
    }

    /// Sets the grapheme symbol of this cell and clears any wide continuation marker.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Cell;
    ///
    /// let mut c = Cell::EMPTY;
    /// c.set_symbol("❯");
    /// assert_eq!(c.symbol(), "❯");
    /// ```
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Self {
        if symbol.is_empty() {
            self.symbol = CompactString::const_new(" ");
        } else {
            self.symbol.clear();
            self.symbol.push_str(symbol);
        }
        self.wide_continuation = false;
        self
    }

    /// Sets the grapheme symbol from a single `char` without heap allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Cell;
    ///
    /// let mut c = Cell::EMPTY;
    /// c.set_char('■');
    /// assert_eq!(c.symbol(), "■");
    /// ```
    pub fn set_char(&mut self, ch: char) -> &mut Self {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        self.symbol.clear();
        self.symbol.push_str(encoded);
        self.wide_continuation = false;
        self
    }

    /// Patches the cell's [`Style`] with `style`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Cell, Color, Style};
    ///
    /// let mut c = Cell::EMPTY;
    /// c.set_style(Style::new().fg(Color::Yellow));
    /// assert_eq!(c.style().foreground(), Some(Color::Yellow));
    /// ```
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = self.style.patch(style);
        self
    }

    /// Replaces the cell's [`Style`] wholesale.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Cell, Color, Style};
    ///
    /// let mut c = Cell::new("x", Style::new().fg(Color::Red).bold());
    /// c.replace_style(Style::new().fg(Color::Green));
    /// assert_eq!(c.style(), Style::new().fg(Color::Green));
    /// ```
    pub fn replace_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    /// Marks or unmarks this cell as the trailing column of a 2-column wide grapheme.
    pub(crate) fn set_wide_continuation(&mut self, is_continuation: bool) {
        if is_continuation {
            self.symbol = CompactString::const_new("");
        } else if self.symbol.is_empty() {
            self.symbol = CompactString::const_new(" ");
        }
        self.wide_continuation = is_continuation;
    }

    /// Resets the cell back to a blank space with `Style::DEFAULT`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Cell, Color, Style};
    ///
    /// let mut c = Cell::new("X", Style::new().fg(Color::Red));
    /// c.reset();
    /// assert_eq!(c, Cell::EMPTY);
    /// ```
    pub fn reset(&mut self) {
        self.symbol = CompactString::const_new(" ");
        self.style = Style::DEFAULT;
        self.wide_continuation = false;
    }
}
