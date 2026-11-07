//! Single-line grapheme-cursor text editor (`TextInputState`, `TextInput`).
//!
//! Synthesizes `InputBox` from `ref/dashboard/de_widgets.go` and `btop`'s
//! grapheme-windowed `TextEdit`:
//! - Supports Left/Right/Home/End cursor movement along Unicode grapheme boundaries.
//! - Supports Backspace, Delete, and Clear (`Ctrl+U`).
//! - Automatically scrolls horizontally so the cursor block (`█`) always remains
//!   inside the visible input box width.
//! - Supports masked secret mode (`'•'`) for passwords and encryption passphrases.

use unicode_segmentation::UnicodeSegmentation;

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::grapheme::display_width;
use crate::core::palette::ColorPalette;
use crate::core::style::Style;

/// Stateful grapheme-cursor buffer for single-line text editing.
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::TextInputState;
///
/// let mut state = TextInputState::from_text("vault");
/// state.insert_char('!');
/// assert_eq!(state.value(), "vault!");
/// state.move_left();
/// state.backspace();
/// assert_eq!(state.value(), "vaul!");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextInputState {
    value: String,
    /// Cursor position measured in grapheme clusters (`0..=grapheme_count`).
    cursor_grapheme: usize,
}

impl TextInputState {
    /// Creates an empty [`TextInputState`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let state = TextInputState::new();
    /// assert!(state.is_empty());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            value: String::new(),
            cursor_grapheme: 0,
        }
    }

    /// Creates a [`TextInputState`] initialized with `initial` and cursor at the end.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let state = TextInputState::from_text("hello");
    /// assert_eq!(state.value(), "hello");
    /// assert_eq!(state.cursor_grapheme(), 5);
    /// ```
    #[must_use]
    pub fn from_text(initial: impl Into<String>) -> Self {
        let value = initial.into();
        let cursor_grapheme = value.graphemes(true).count();
        Self {
            value,
            cursor_grapheme,
        }
    }

    /// Returns the current UTF-8 string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let state = TextInputState::from_text("abc");
    /// assert_eq!(state.value(), "abc");
    /// ```
    #[must_use]
    #[inline]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns `true` if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// assert!(TextInputState::new().is_empty());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Returns the cursor position in grapheme clusters.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// assert_eq!(TextInputState::from_text("hi").cursor_grapheme(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn cursor_grapheme(&self) -> usize {
        self.cursor_grapheme
    }

    /// Replaces the text content and moves the cursor to the end.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::new();
    /// state.set_value("new");
    /// assert_eq!(state.value(), "new");
    /// ```
    pub fn set_value(&mut self, text: impl Into<String>) {
        self.value = text.into();
        self.cursor_grapheme = self.value.graphemes(true).count();
    }

    /// Clears all text and resets the cursor to `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("secret");
    /// state.clear();
    /// assert!(state.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_grapheme = 0;
    }

    /// Inserts a character at the current cursor position.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.move_left();
    /// state.insert_char('X');
    /// assert_eq!(state.value(), "aXb");
    /// ```
    pub fn insert_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        let byte_idx = self.byte_index_of_grapheme(self.cursor_grapheme);
        self.value.insert(byte_idx, ch);
        self.cursor_grapheme = self
            .value
            .graphemes(true)
            .count()
            .min(self.cursor_grapheme + 1);
    }

    /// Deletes the grapheme immediately preceding the cursor (`Backspace`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.backspace();
    /// assert_eq!(state.value(), "a");
    /// ```
    pub fn backspace(&mut self) {
        if self.cursor_grapheme == 0 {
            return;
        }
        let start_byte = self.byte_index_of_grapheme(self.cursor_grapheme - 1);
        let end_byte = self.byte_index_of_grapheme(self.cursor_grapheme);
        self.value.replace_range(start_byte..end_byte, "");
        self.cursor_grapheme -= 1;
    }

    /// Deletes the grapheme directly under the cursor (`Delete`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.move_home();
    /// state.delete();
    /// assert_eq!(state.value(), "b");
    /// ```
    pub fn delete(&mut self) {
        let count = self.value.graphemes(true).count();
        if self.cursor_grapheme >= count {
            return;
        }
        let start_byte = self.byte_index_of_grapheme(self.cursor_grapheme);
        let end_byte = self.byte_index_of_grapheme(self.cursor_grapheme + 1);
        self.value.replace_range(start_byte..end_byte, "");
    }

    /// Moves the cursor one grapheme to the left.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.move_left();
    /// assert_eq!(state.cursor_grapheme(), 1);
    /// ```
    #[inline]
    pub const fn move_left(&mut self) {
        self.cursor_grapheme = self.cursor_grapheme.saturating_sub(1);
    }

    /// Moves the cursor one grapheme to the right.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.move_home();
    /// state.move_right();
    /// assert_eq!(state.cursor_grapheme(), 1);
    /// ```
    #[inline]
    pub fn move_right(&mut self) {
        let count = self.value.graphemes(true).count();
        if self.cursor_grapheme < count {
            self.cursor_grapheme += 1;
        }
    }

    /// Moves the cursor to the start of the input (`Home`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.move_home();
    /// assert_eq!(state.cursor_grapheme(), 0);
    /// ```
    #[inline]
    pub const fn move_home(&mut self) {
        self.cursor_grapheme = 0;
    }

    /// Moves the cursor to the end of the input (`End`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TextInputState;
    ///
    /// let mut state = TextInputState::from_text("ab");
    /// state.move_home();
    /// state.move_end();
    /// assert_eq!(state.cursor_grapheme(), 2);
    /// ```
    #[inline]
    pub fn move_end(&mut self) {
        self.cursor_grapheme = self.value.graphemes(true).count();
    }

    fn byte_index_of_grapheme(&self, grapheme_idx: usize) -> usize {
        self.value
            .grapheme_indices(true)
            .nth(grapheme_idx)
            .map_or(self.value.len(), |(byte_idx, _)| byte_idx)
    }
}

/// Bordered 3-row input box widget (`╭───╮`, `│ text█ │`, `╰───╯`).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
/// use tui_rs::widgets::{TextInput, TextInputState};
///
/// let state = TextInputState::from_text("vault.enc");
/// let area = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(3));
/// let mut buf = Buffer::empty(area);
/// TextInput::new(&state)
///     .render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 30, &ColorPalette::DEFAULT);
/// assert!(buf.to_visual_lines()[1].contains("vault.enc"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TextInput<'a> {
    state: &'a TextInputState,
    masked: bool,
    tty_mode: bool,
}

impl<'a> TextInput<'a> {
    /// Creates a new [`TextInput`] widget bound to `state`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::{TextInput, TextInputState};
    ///
    /// let state = TextInputState::new();
    /// let input = TextInput::new(&state);
    /// assert!(!input.is_masked());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(state: &'a TextInputState) -> Self {
        Self {
            state,
            masked: false,
            tty_mode: false,
        }
    }

    /// Configures whether input characters are masked with `'•'` (or `'*'` in TTY mode).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::{TextInput, TextInputState};
    ///
    /// let state = TextInputState::new();
    /// let input = TextInput::new(&state).with_masked(true);
    /// assert!(input.is_masked());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Configures whether TTY-safe corners (`┌ ┐ └ ┘`) and mask (`'*'`) are used.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::{TextInput, TextInputState};
    ///
    /// let state = TextInputState::new();
    /// let input = TextInput::new(&state).with_tty_mode(true);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns `true` if secret masking is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::{TextInput, TextInputState};
    ///
    /// let state = TextInputState::new();
    /// assert!(!TextInput::new(&state).is_masked());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_masked(&self) -> bool {
        self.masked
    }

    /// Renders the 3-row bordered input box at `(col, row)` across `width` columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    /// use tui_rs::widgets::{TextInput, TextInputState};
    ///
    /// let state = TextInputState::from_text("secret");
    /// let area = Rect::from_size(TerminalWidth::new(25), TerminalHeight::new(3));
    /// let mut buf = Buffer::empty(area);
    /// TextInput::new(&state)
    ///     .with_masked(true)
    ///     .render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 25, &ColorPalette::DEFAULT);
    /// assert!(buf.to_visual_lines()[1].contains('•'));
    /// ```
    pub fn render(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        width: u16,
        palette: &ColorPalette,
    ) {
        if width < 6 {
            return;
        }
        let inner_w = usize::from(width.saturating_sub(4));
        let border_style = Style::new().with_fg(palette.border_focus);
        let text_style = Style::new().with_fg(palette.text).bold();
        let cursor_style = Style::new().with_fg(palette.accent2).bold();

        let (tl, tr, bl, br, rule) = if self.tty_mode {
            ("┌", "┐", "└", "┘", "-")
        } else {
            ("╭", "╮", "╰", "╯", "─")
        };

        // Row 0: top border
        let _ = buffer.set_cell(col, row, tl, border_style);
        for i in 1..width.saturating_sub(1) {
            let _ = buffer.set_cell(col.saturating_add(i), row, rule, border_style);
        }
        let _ = buffer.set_cell(col.saturating_add(width - 1), row, tr, border_style);

        // Row 1: content row `│ <windowed text + cursor> │`
        let mid_row = row.saturating_add(1);
        let _ = buffer.set_cell(col, mid_row, "│", border_style);
        let _ = buffer.set_cell(col.saturating_add(1), mid_row, " ", text_style);
        let _ = buffer.set_cell(col.saturating_add(width - 2), mid_row, " ", text_style);
        let _ = buffer.set_cell(col.saturating_add(width - 1), mid_row, "│", border_style);

        // Compute horizontal grapheme window around cursor_grapheme
        let graphemes: Vec<&str> = self.state.value.graphemes(true).collect();
        let total_g = graphemes.len();
        let cur_g = self.state.cursor_grapheme.min(total_g);

        let text_budget = inner_w.saturating_sub(1); // Reserve 1 cell for cursor block '█'
        let mut start_g = 0;
        let mut end_g = total_g;

        if total_g > text_budget {
            start_g = cur_g.saturating_sub(text_budget / 2);
            if start_g + text_budget > total_g {
                start_g = total_g.saturating_sub(text_budget);
            }
            end_g = (start_g + text_budget).min(total_g);
        }

        let mask_glyph = if self.tty_mode { "*" } else { "•" };
        let cursor_glyph = if self.tty_mode { "_" } else { "█" };
        let mut cx = col.saturating_add(2);
        let max_cx = col.get().saturating_add(width - 2);

        for (idx, &g) in graphemes.iter().enumerate().take(end_g).skip(start_g) {
            if idx == cur_g && cx.get() < max_cx {
                let _ = buffer.set_cell(cx, mid_row, cursor_glyph, cursor_style);
                cx = cx.saturating_add(1);
            }
            let glyph = if self.masked { mask_glyph } else { g };
            let gw = display_width(glyph).as_u16();
            if cx.get().saturating_add(gw) <= max_cx {
                cx = buffer.set_string(cx, mid_row, glyph, text_style, max_cx - cx.get());
            }
        }
        if cur_g >= end_g && cx.get() < max_cx {
            let _ = buffer.set_cell(cx, mid_row, cursor_glyph, cursor_style);
        }

        // Row 2: bottom border
        let bot_row = row.saturating_add(2);
        let _ = buffer.set_cell(col, bot_row, bl, border_style);
        for i in 1..width.saturating_sub(1) {
            let _ = buffer.set_cell(col.saturating_add(i), bot_row, rule, border_style);
        }
        let _ = buffer.set_cell(col.saturating_add(width - 1), bot_row, br, border_style);
    }
}
