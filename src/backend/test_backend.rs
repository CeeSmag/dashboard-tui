//! Headless in-memory [`TestBackend`] for deterministic snapshot testing (`RULES.md` §6.1).
//!
//! Allows every widget, layout solver, modal, and full dashboard page to be rendered and
//! verified without a physical TTY.

use crate::backend::{Backend, TerminalCapabilities};
use crate::core::{Buffer, CellDiff, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
use crate::error::TuiError;

/// In-memory headless backend implementing [`Backend`] for unit and snapshot tests.
///
/// # Examples
///
/// ```
/// use tui_rs::backend::{Backend, TestBackend};
/// use tui_rs::core::{TerminalHeight, TerminalWidth};
///
/// let backend = TestBackend::new(TerminalWidth(40), TerminalHeight(10));
/// assert_eq!(backend.dimensions(), (TerminalWidth(40), TerminalHeight(10)));
/// assert_eq!(backend.screen_buffer().to_visual_lines().len(), 10);
/// ```
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "TestBackend tracks 4 independent terminal hardware mode flags"
)]
pub struct TestBackend {
    width: TerminalWidth,
    height: TerminalHeight,
    screen_buffer: Buffer,
    capabilities: TerminalCapabilities,
    raw_mode_enabled: bool,
    alternate_screen_active: bool,
    alternate_scroll_active: bool,
    cursor_visible: bool,
    cursor_position: (ColumnIndex, RowIndex),
    window_title: String,
    flush_count: usize,
}

impl TestBackend {
    /// Creates a new headless `TestBackend` with the specified dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    ///
    /// let tb = TestBackend::new(TerminalWidth(80), TerminalHeight(24));
    /// assert!(!tb.is_raw_mode_enabled());
    /// ```
    #[must_use]
    pub fn new(width: TerminalWidth, height: TerminalHeight) -> Self {
        let bounds = Rect::from_dimensions(width, height);
        Self {
            width,
            height,
            screen_buffer: Buffer::empty(bounds),
            capabilities: TerminalCapabilities::default(),
            raw_mode_enabled: false,
            alternate_screen_active: false,
            alternate_scroll_active: false,
            cursor_visible: true,
            cursor_position: (ColumnIndex::ZERO, RowIndex::ZERO),
            window_title: String::new(),
            flush_count: 0,
        }
    }

    /// Creates a `TestBackend` configured to simulate a 16-color Linux virtual console (`TTY`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{Backend, TestBackend};
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    ///
    /// let tb = TestBackend::new_tty(TerminalWidth(80), TerminalHeight(25));
    /// assert!(tb.capabilities().is_tty_console());
    /// ```
    #[must_use]
    pub fn new_tty(width: TerminalWidth, height: TerminalHeight) -> Self {
        let mut tb = Self::new(width, height);
        tb.capabilities =
            TerminalCapabilities::new(crate::core::ColorDepth::Ansi16, true, false, false);
        tb
    }

    /// Simulates a terminal resize event (`SIGWINCH`), resizing the internal screen buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{Backend, TestBackend};
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    ///
    /// let mut tb = TestBackend::new(TerminalWidth(80), TerminalHeight(24));
    /// tb.resize(TerminalWidth(100), TerminalHeight(30));
    /// assert_eq!(tb.dimensions(), (TerminalWidth(100), TerminalHeight(30)));
    /// ```
    pub fn resize(&mut self, width: TerminalWidth, height: TerminalHeight) {
        self.width = width;
        self.height = height;
        self.screen_buffer
            .resize(Rect::from_dimensions(width, height));
    }

    /// Returns a reference to the current headless screen [`Buffer`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    ///
    /// let tb = TestBackend::new(TerminalWidth(10), TerminalHeight(2));
    /// assert_eq!(tb.screen_buffer().bounds().width(), TerminalWidth(10));
    /// ```
    #[must_use]
    pub const fn screen_buffer(&self) -> &Buffer {
        &self.screen_buffer
    }

    /// Returns the rendered screen as a newline-joined visual string for `insta` snapshot assertions.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    ///
    /// let tb = TestBackend::new(TerminalWidth(4), TerminalHeight(2));
    /// assert_eq!(tb.to_visual_grid(), "    \n    ");
    /// ```
    #[must_use]
    pub fn to_visual_grid(&self) -> String {
        self.screen_buffer.to_visual_lines().join("\n")
    }

    /// Returns true if raw mode is currently engaged on this test backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    /// assert!(!TestBackend::new(TerminalWidth(10), TerminalHeight(2)).is_raw_mode_enabled());
    /// ```
    #[must_use]
    pub const fn is_raw_mode_enabled(&self) -> bool {
        self.raw_mode_enabled
    }

    /// Returns true if the alternate screen buffer is currently active.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    /// assert!(!TestBackend::new(TerminalWidth(10), TerminalHeight(2)).is_alternate_screen_active());
    /// ```
    #[must_use]
    pub const fn is_alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
    }

    /// Returns true if DEC Mode 1007 alternate scroll is currently active.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    /// assert!(!TestBackend::new(TerminalWidth(10), TerminalHeight(2)).is_alternate_scroll_active());
    /// ```
    #[must_use]
    pub const fn is_alternate_scroll_active(&self) -> bool {
        self.alternate_scroll_active
    }

    /// Returns true if the terminal cursor is currently visible.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    /// assert!(TestBackend::new(TerminalWidth(10), TerminalHeight(2)).is_cursor_visible());
    /// ```
    #[must_use]
    pub const fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Returns the hardware cursor position last set through the backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{Backend, TestBackend};
    /// use tui_rs::core::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
    ///
    /// let mut backend = TestBackend::new(TerminalWidth(10), TerminalHeight(2));
    /// backend.set_cursor_position(ColumnIndex(3), RowIndex(1)).unwrap();
    /// assert_eq!(backend.cursor_position(), (ColumnIndex(3), RowIndex(1)));
    /// ```
    #[must_use]
    pub const fn cursor_position(&self) -> (ColumnIndex, RowIndex) {
        self.cursor_position
    }

    /// Returns the last set window title.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    /// assert_eq!(TestBackend::new(TerminalWidth(10), TerminalHeight(2)).window_title(), "");
    /// ```
    #[must_use]
    pub fn window_title(&self) -> &str {
        &self.window_title
    }

    /// Returns the number of atomic frame flushes performed so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TestBackend;
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    /// assert_eq!(TestBackend::new(TerminalWidth(10), TerminalHeight(2)).flush_count(), 0);
    /// ```
    #[must_use]
    pub const fn flush_count(&self) -> usize {
        self.flush_count
    }
}

impl Backend for TestBackend {
    fn dimensions(&self) -> (TerminalWidth, TerminalHeight) {
        (self.width, self.height)
    }

    fn capabilities(&self) -> TerminalCapabilities {
        self.capabilities
    }

    fn enable_raw_mode(&mut self) -> Result<(), TuiError> {
        self.raw_mode_enabled = true;
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> Result<(), TuiError> {
        self.raw_mode_enabled = false;
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> Result<(), TuiError> {
        self.alternate_screen_active = true;
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> Result<(), TuiError> {
        self.alternate_screen_active = false;
        Ok(())
    }

    fn enable_alternate_scroll(&mut self) -> Result<(), TuiError> {
        self.alternate_scroll_active = true;
        Ok(())
    }

    fn disable_alternate_scroll(&mut self) -> Result<(), TuiError> {
        self.alternate_scroll_active = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), TuiError> {
        self.cursor_visible = true;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), TuiError> {
        self.cursor_visible = false;
        Ok(())
    }

    fn set_cursor_position(&mut self, column: ColumnIndex, row: RowIndex) -> Result<(), TuiError> {
        self.cursor_position = (column, row);
        Ok(())
    }

    fn scroll_up(&mut self, lines: u16) -> Result<(), TuiError> {
        let rows = self.height.get();
        let shift = lines.min(rows);
        for row in 0..rows {
            for column in 0..self.width.get() {
                let source = row.saturating_add(shift);
                let moved = if source < rows {
                    self.screen_buffer
                        .cell_at(ColumnIndex(column), RowIndex(source))
                        .cloned()
                } else {
                    None
                };
                if let Some(cell) = self
                    .screen_buffer
                    .cell_at_mut(ColumnIndex(column), RowIndex(row))
                {
                    *cell = moved.unwrap_or_default();
                }
            }
        }
        Ok(())
    }

    fn clear_screen(&mut self) -> Result<(), TuiError> {
        self.screen_buffer.reset();
        Ok(())
    }

    fn set_window_title(&mut self, title: &str) -> Result<(), TuiError> {
        self.window_title.clear();
        self.window_title.push_str(title);
        Ok(())
    }

    fn draw_frame<'a>(
        &mut self,
        diffs: impl IntoIterator<Item = CellDiff<'a>>,
    ) -> Result<(), TuiError> {
        for diff in diffs {
            let col = diff.column();
            let row = diff.row();
            let src = diff.cell();
            let gw = crate::core::grapheme_width(src.symbol());
            if let Some(dst) = self.screen_buffer.cell_at_mut(col, row) {
                *dst = src.clone();
            }
            if gw == 2 {
                if let Some(next_dst) = self.screen_buffer.cell_at_mut(col.saturating_add(1), row) {
                    next_dst.replace_style(src.style());
                    next_dst.set_wide_continuation(true);
                }
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), TuiError> {
        self.flush_count = self.flush_count.saturating_add(1);
        Ok(())
    }
}
