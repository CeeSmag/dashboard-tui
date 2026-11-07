//! Production OS terminal driver (`CrosstermBackend`) using batched single-write ANSI flushing.
//!
//! Never flushes per cell or per widget (`RULES.md` §3.3): all CSI/SGR escape sequences
//! are serialized into [`AnsiFrameWriter`] and written to the underlying writer in one atomic flush.

use std::io::Write;

use crate::backend::ansi_writer::{
    ALT_SCROLL_ENABLE, ALT_SCROLL_RESTORE, AnsiFrameWriter, GRAPHEME_CLUSTERING_DISABLE,
    GRAPHEME_CLUSTERING_ENABLE,
};
use crate::backend::{Backend, TerminalCapabilities};
use crate::core::{CellDiff, ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
use crate::error::TuiError;

/// Production terminal driver wrapping any [`std::io::Write`] stream (such as `std::io::Stdout`).
///
/// # Examples
///
/// ```
/// use tui_rs::backend::{Backend, CrosstermBackend};
///
/// let backend = CrosstermBackend::with_writer(Vec::<u8>::new());
/// let (w, h) = backend.dimensions();
/// assert!(!w.is_zero() && !h.is_zero());
/// ```
#[derive(Debug)]
pub struct CrosstermBackend<W: Write> {
    writer: W,
    frame_writer: AnsiFrameWriter,
    capabilities: TerminalCapabilities,
    fallback_dimensions: (TerminalWidth, TerminalHeight),
}

impl<W: Write> CrosstermBackend<W> {
    /// Creates a new `CrosstermBackend` with automatic capability detection and a 64 KiB frame buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::CrosstermBackend;
    ///
    /// let backend = CrosstermBackend::with_writer(Vec::<u8>::new());
    /// assert!(backend.writer().is_empty());
    /// ```
    #[must_use]
    pub fn with_writer(writer: W) -> Self {
        let capabilities = TerminalCapabilities::detect();
        Self::with_capabilities(writer, capabilities)
    }

    /// Creates a new `CrosstermBackend` with explicit [`TerminalCapabilities`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{Backend, CrosstermBackend, TerminalCapabilities};
    ///
    /// let backend = CrosstermBackend::with_capabilities(Vec::<u8>::new(), TerminalCapabilities::default());
    /// assert_eq!(backend.capabilities(), TerminalCapabilities::default());
    /// ```
    #[must_use]
    pub fn with_capabilities(writer: W, capabilities: TerminalCapabilities) -> Self {
        Self {
            writer,
            frame_writer: AnsiFrameWriter::with_capacity(
                65_536,
                capabilities.color_depth(),
                capabilities.supports_synchronized_output(),
            ),
            capabilities,
            fallback_dimensions: (TerminalWidth(80), TerminalHeight(24)),
        }
    }

    /// Returns a shared reference to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::CrosstermBackend;
    /// let b = CrosstermBackend::with_writer(Vec::<u8>::new());
    /// assert!(b.writer().is_empty());
    /// ```
    #[must_use]
    pub const fn writer(&self) -> &W {
        &self.writer
    }

    /// Consumes the backend and returns the underlying writer (`C-CONV` `into_writer`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::CrosstermBackend;
    /// let b = CrosstermBackend::with_writer(Vec::<u8>::new());
    /// let out = b.into_writer();
    /// assert!(out.is_empty());
    /// ```
    #[must_use]
    pub fn into_writer(self) -> W {
        self.writer
    }
}

impl<W: Write> Backend for CrosstermBackend<W> {
    fn dimensions(&self) -> (TerminalWidth, TerminalHeight) {
        #[cfg(feature = "crossterm")]
        {
            if let Ok((cols, rows)) = crossterm::terminal::size() {
                if cols > 0 && rows > 0 {
                    return (TerminalWidth(cols), TerminalHeight(rows));
                }
            }
        }
        self.fallback_dimensions
    }

    fn capabilities(&self) -> TerminalCapabilities {
        self.capabilities
    }

    fn enable_raw_mode(&mut self) -> Result<(), TuiError> {
        #[cfg(feature = "crossterm")]
        {
            crossterm::terminal::enable_raw_mode()?;
        }
        if self.capabilities.supports_grapheme_clustering() {
            self.writer.write_all(GRAPHEME_CLUSTERING_ENABLE)?;
        }
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> Result<(), TuiError> {
        if self.capabilities.supports_grapheme_clustering() {
            let _ = self.writer.write_all(GRAPHEME_CLUSTERING_DISABLE);
        }
        #[cfg(feature = "crossterm")]
        {
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(b"\x1b[?1049h\x1b[2J\x1b[H")?;
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(b"\x1b[0m\x1b[?1049l")?;
        Ok(())
    }

    fn enable_alternate_scroll(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(ALT_SCROLL_ENABLE)?;
        Ok(())
    }

    fn disable_alternate_scroll(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(ALT_SCROLL_RESTORE)?;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(b"\x1b[?25h")?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(b"\x1b[?25l")?;
        Ok(())
    }

    fn set_cursor_position(&mut self, column: ColumnIndex, row: RowIndex) -> Result<(), TuiError> {
        write!(
            self.writer,
            "\x1b[{};{}H",
            row.as_u16().saturating_add(1),
            column.as_u16().saturating_add(1)
        )?;
        Ok(())
    }

    fn clear_screen(&mut self) -> Result<(), TuiError> {
        self.writer.write_all(b"\x1b[0m\x1b[2J\x1b[H")?;
        Ok(())
    }

    fn set_window_title(&mut self, title: &str) -> Result<(), TuiError> {
        self.frame_writer.write_window_title(title);
        self.writer.write_all(self.frame_writer.as_bytes())?;
        Ok(())
    }

    fn draw_frame<'a>(
        &mut self,
        diffs: impl IntoIterator<Item = CellDiff<'a>>,
    ) -> Result<(), TuiError> {
        let bytes = self.frame_writer.encode_frame(diffs);
        if !bytes.is_empty() {
            self.writer.write_all(bytes)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), TuiError> {
        self.writer.flush()?;
        Ok(())
    }
}
