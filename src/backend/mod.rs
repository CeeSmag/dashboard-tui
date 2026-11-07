//! Abstract terminal driver boundary (`Backend`) and capability probes (`RULES.md` §5.3).
//!
//! Decouples all widget layout, styling, and dashboard engine logic from concrete
//! OS terminal drivers so applications can swap drivers or run headless tests via [`TestBackend`].

pub mod ansi_writer;
pub mod crossterm_backend;
pub mod test_backend;
pub mod tty_detector;

pub use self::ansi_writer::{
    ALT_SCROLL_ENABLE, ALT_SCROLL_RESTORE, AnsiFrameWriter, GRAPHEME_CLUSTERING_DISABLE,
    GRAPHEME_CLUSTERING_ENABLE, ProgressBarState, SYNC_UPDATE_BEGIN, SYNC_UPDATE_END,
    format_hyperlink, format_hyperlink_with_id,
};
pub use self::crossterm_backend::CrosstermBackend;
pub use self::test_backend::TestBackend;
pub use self::tty_detector::{TerminalCapabilities, detect_color_depth, detect_linux_tty_console};

use crate::core::{CellDiff, ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
use crate::error::TuiError;

/// Hardware abstraction trait for terminal display surfaces and headless test buffers.
///
/// # Examples
///
/// ```
/// use tui_rs::backend::{Backend, TestBackend};
/// use tui_rs::core::{TerminalHeight, TerminalWidth};
///
/// let mut backend = TestBackend::new(TerminalWidth(80), TerminalHeight(24));
/// backend.hide_cursor().unwrap();
/// assert!(!backend.is_cursor_visible());
/// ```
pub trait Backend {
    /// Queries the current `(width, height)` of the terminal screen in cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{Backend, TestBackend};
    /// use tui_rs::core::{TerminalHeight, TerminalWidth};
    ///
    /// let b = TestBackend::new(TerminalWidth(60), TerminalHeight(20));
    /// assert_eq!(b.dimensions(), (TerminalWidth(60), TerminalHeight(20)));
    /// ```
    fn dimensions(&self) -> (TerminalWidth, TerminalHeight);

    /// Returns the detected or configured [`TerminalCapabilities`] of this backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{Backend, TestBackend};
    /// use tui_rs::core::{ColorDepth, TerminalHeight, TerminalWidth};
    ///
    /// let b = TestBackend::new(TerminalWidth(60), TerminalHeight(20));
    /// assert_eq!(b.capabilities().color_depth(), ColorDepth::TrueColor);
    /// ```
    fn capabilities(&self) -> TerminalCapabilities;

    /// Enables terminal raw mode (disabling canonical line buffering and local echo).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if the underlying OS terminal ioctl fails.
    fn enable_raw_mode(&mut self) -> Result<(), TuiError>;

    /// Restores canonical terminal mode.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if the underlying OS terminal ioctl fails.
    fn disable_raw_mode(&mut self) -> Result<(), TuiError>;

    /// Switches to the alternate screen buffer (`DEC ?1049h`).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn enter_alternate_screen(&mut self) -> Result<(), TuiError>;

    /// Exits the alternate screen buffer and restores the main screen (`DEC ?1049l`).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn leave_alternate_screen(&mut self) -> Result<(), TuiError>;

    /// Enables DEC Private Mode 1007 (`\x1b[?1007s\x1b[?1007h`) so mouse wheel scrolls emit arrow keys
    /// while native terminal click-drag selection remains active.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn enable_alternate_scroll(&mut self) -> Result<(), TuiError>;

    /// Restores DEC Private Mode 1007 (`\x1b[?1007r`).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn disable_alternate_scroll(&mut self) -> Result<(), TuiError>;

    /// Shows the terminal text cursor (`DEC ?25h`).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn show_cursor(&mut self) -> Result<(), TuiError>;

    /// Hides the terminal text cursor (`DEC ?25l`).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn hide_cursor(&mut self) -> Result<(), TuiError>;

    /// Moves the hardware cursor to `(column, row)`.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn set_cursor_position(&mut self, column: ColumnIndex, row: RowIndex) -> Result<(), TuiError>;

    /// Clears the entire terminal screen and resets SGR attributes.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn clear_screen(&mut self) -> Result<(), TuiError>;

    /// Updates the terminal emulator window title (`OSC 2`).
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing to the terminal fails.
    fn set_window_title(&mut self, title: &str) -> Result<(), TuiError>;

    /// Renders a batch of mutated [`CellDiff`] cells for the current frame.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if writing the frame byte buffer fails.
    fn draw_frame<'a>(
        &mut self,
        diffs: impl IntoIterator<Item = CellDiff<'a>>,
    ) -> Result<(), TuiError>;

    /// Atomically flushes buffered output to the underlying OS stream.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if flushing the stream fails.
    fn flush(&mut self) -> Result<(), TuiError>;
}
