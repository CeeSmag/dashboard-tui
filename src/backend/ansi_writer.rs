//! Batched, zero-per-cell-flush ANSI/CSI escape sequence frame serializer (`AnsiFrameWriter`).
//!
//! Serializes all cursor movements, SGR style transitions, and DEC Private Mode 2026
//! synchronized output markers into a single pre-allocated `Vec<u8>` buffer (`RULES.md` §3.3).

use crate::core::color::{Color, ColorDepth};
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::diff::CellDiff;
use crate::core::grapheme::display_width;
use crate::core::style::{Modifiers, Style};

/// Escape sequence to begin a DEC Mode 2026 synchronized frame update (`btop` / `bubbletea`).
pub const SYNC_UPDATE_BEGIN: &[u8] = b"\x1b[?2026h";
/// Escape sequence to end a DEC Mode 2026 synchronized frame update.
pub const SYNC_UPDATE_END: &[u8] = b"\x1b[?2026l";
/// Escape sequence to save and enable DEC Mode 1007 Alternate Scroll (`de_run.go`).
pub const ALT_SCROLL_ENABLE: &[u8] = b"\x1b[?1007s\x1b[?1007h";
/// Escape sequence to restore DEC Mode 1007 Alternate Scroll.
pub const ALT_SCROLL_RESTORE: &[u8] = b"\x1b[?1007r";
/// Escape sequence to enable DEC Mode 2027 Unicode Core Grapheme Clustering (`bubbletea`).
pub const GRAPHEME_CLUSTERING_ENABLE: &[u8] = b"\x1b[?2027h";
/// Escape sequence to disable DEC Mode 2027 Unicode Core Grapheme Clustering.
pub const GRAPHEME_CLUSTERING_DISABLE: &[u8] = b"\x1b[?2027l";

/// State of the OSC `9;4` native terminal progress bar (`bubbletea` `ProgressBar`).
///
/// # Examples
///
/// ```
/// use tui_rs::backend::ProgressBarState;
/// assert_eq!(ProgressBarState::default(), ProgressBarState::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProgressBarState {
    /// Progress bar hidden/reset (`\x1b]9;4;0\x07`).
    #[default]
    None,
    /// Normal progress percentage (`0..=100`).
    Normal(u8),
    /// Error state progress percentage (`0..=100`).
    Error(u8),
    /// Indeterminate pulsing progress bar.
    Indeterminate,
    /// Warning state progress percentage (`0..=100`).
    Warning(u8),
}

/// Pre-allocated byte buffer that encodes a frame's [`CellDiff`] iterator into minimal ANSI sequences.
///
/// # Examples
///
/// ```
/// use tui_rs::backend::AnsiFrameWriter;
/// use tui_rs::core::{Buffer, Color, ColorDepth, ColumnIndex, diff_buffers, Rect, RowIndex, Style};
///
/// let rect = Rect::from_xywh(0, 0, 10, 2);
/// let prev = Buffer::empty(rect);
/// let mut curr = Buffer::empty(rect);
/// curr.set_str_clipped(ColumnIndex(0), RowIndex(0), "OK", ColumnIndex(10), Style::new().fg(Color::Green));
///
/// let mut writer = AnsiFrameWriter::with_capacity(1024, ColorDepth::TrueColor, true);
/// let bytes = writer.encode_frame(diff_buffers(&prev, &curr));
/// assert!(!bytes.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct AnsiFrameWriter {
    buffer: Vec<u8>,
    color_depth: ColorDepth,
    synchronized_output: bool,
}

impl AnsiFrameWriter {
    /// Creates a new `AnsiFrameWriter` with pre-allocated byte capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::AnsiFrameWriter;
    /// use tui_rs::core::ColorDepth;
    ///
    /// let writer = AnsiFrameWriter::with_capacity(4096, ColorDepth::TrueColor, true);
    /// assert!(writer.as_bytes().is_empty());
    /// ```
    #[must_use]
    pub fn with_capacity(
        capacity: usize,
        color_depth: ColorDepth,
        synchronized_output: bool,
    ) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            color_depth,
            synchronized_output,
        }
    }

    /// Returns a read-only view of the currently encoded frame bytes (`C-CONV` `as_bytes`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::AnsiFrameWriter;
    /// use tui_rs::core::ColorDepth;
    ///
    /// let writer = AnsiFrameWriter::with_capacity(64, ColorDepth::TrueColor, false);
    /// assert_eq!(writer.as_bytes(), b"");
    /// ```
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Encodes an iterator of [`CellDiff`] items into the internal byte buffer and returns a slice.
    ///
    /// Skips redundant cursor movements when cells are horizontally contiguous and skips redundant
    /// SGR sequences when consecutive cells share the same degraded [`Style`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::AnsiFrameWriter;
    /// use tui_rs::core::{Buffer, ColorDepth, diff_buffers, Rect};
    ///
    /// let r = Rect::from_xywh(0, 0, 4, 1);
    /// let a = Buffer::empty(r);
    /// let b = Buffer::empty(r);
    /// let mut writer = AnsiFrameWriter::with_capacity(128, ColorDepth::TrueColor, true);
    /// // Zero diffs produce zero bytes:
    /// assert!(writer.encode_frame(diff_buffers(&a, &b)).is_empty());
    /// ```
    pub fn encode_frame<'a>(
        &mut self,
        diffs: impl IntoIterator<Item = CellDiff<'a>>,
    ) -> &[u8] {
        self.buffer.clear();
        let mut iter = diffs.into_iter().peekable();
        if iter.peek().is_none() {
            return &self.buffer;
        }

        if self.synchronized_output {
            self.buffer.extend_from_slice(SYNC_UPDATE_BEGIN);
        }

        let mut last_cursor: Option<(ColumnIndex, RowIndex)> = None;
        let mut active_style: Option<Style> = None;

        for diff in iter {
            let col = diff.column();
            let row = diff.row();
            let cell = diff.cell();

            let needs_move =
                last_cursor.is_none_or(|(expected_col, expected_row)| {
                    expected_col != col || expected_row != row
                });

            if needs_move {
                push_cursor_move(&mut self.buffer, col, row);
            }

            let target_style = cell.style().degrade(self.color_depth);
            if active_style != Some(target_style) {
                push_style_sgr(&mut self.buffer, target_style, self.color_depth);
                active_style = Some(target_style);
            }

            let sym = cell.symbol();
            self.buffer.extend_from_slice(sym.as_bytes());

            let step = display_width(sym).as_u16().max(1);
            last_cursor = Some((col.saturating_add(step), row));
        }

        // Always reset SGR attributes at the end of a frame diff batch
        self.buffer.extend_from_slice(b"\x1b[0m");

        if self.synchronized_output {
            self.buffer.extend_from_slice(SYNC_UPDATE_END);
        }

        &self.buffer
    }

    /// Appends an OSC `2` window title update sequence (`\x1b]2;Title\x07`) to the buffer.
    ///
    /// Sanitizes control characters so arbitrary strings cannot inject escape sequences.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::AnsiFrameWriter;
    /// use tui_rs::core::ColorDepth;
    ///
    /// let mut w = AnsiFrameWriter::with_capacity(64, ColorDepth::TrueColor, false);
    /// w.write_window_title("VaultDrive v2.2.0");
    /// assert!(w.as_bytes().starts_with(b"\x1b]2;VaultDrive v2.2.0\x07"));
    /// ```
    pub fn write_window_title(&mut self, title: &str) {
        self.buffer.extend_from_slice(b"\x1b]2;");
        for byte in title.bytes() {
            if (0x20..0x7f).contains(&byte) || byte >= 0x80 {
                self.buffer.push(byte);
            }
        }
        self.buffer.push(0x07);
    }

    /// Appends an OSC `9;4` terminal progress bar sequence (`bubbletea` `setProgressBar`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::{AnsiFrameWriter, ProgressBarState};
    /// use tui_rs::core::ColorDepth;
    ///
    /// let mut w = AnsiFrameWriter::with_capacity(64, ColorDepth::TrueColor, false);
    /// w.write_progress_bar(ProgressBarState::Normal(75));
    /// assert_eq!(w.as_bytes(), b"\x1b]9;4;1;75\x07");
    /// ```
    pub fn write_progress_bar(&mut self, state: ProgressBarState) {
        match state {
            ProgressBarState::None => self.buffer.extend_from_slice(b"\x1b]9;4;0\x07"),
            ProgressBarState::Normal(pct) => {
                self.buffer.extend_from_slice(b"\x1b]9;4;1;");
                push_u16_decimal(&mut self.buffer, u16::from(pct.min(100)));
                self.buffer.push(0x07);
            }
            ProgressBarState::Error(pct) => {
                self.buffer.extend_from_slice(b"\x1b]9;4;2;");
                push_u16_decimal(&mut self.buffer, u16::from(pct.min(100)));
                self.buffer.push(0x07);
            }
            ProgressBarState::Indeterminate => self.buffer.extend_from_slice(b"\x1b]9;4;3\x07"),
            ProgressBarState::Warning(pct) => {
                self.buffer.extend_from_slice(b"\x1b]9;4;4;");
                push_u16_decimal(&mut self.buffer, u16::from(pct.min(100)));
                self.buffer.push(0x07);
            }
        }
    }
}

/// Appends 1-based CSI cursor movement `\x1b[{row+1};{col+1}H` without heap allocation.
fn push_cursor_move(buf: &mut Vec<u8>, col: ColumnIndex, row: RowIndex) {
    buf.extend_from_slice(b"\x1b[");
    push_u16_decimal(buf, row.as_u16().saturating_add(1));
    buf.push(b';');
    push_u16_decimal(buf, col.as_u16().saturating_add(1));
    buf.push(b'H');
}

/// Appends a combined SGR escape sequence `\x1b[0;...m` for `style`.
fn push_style_sgr(buf: &mut Vec<u8>, style: Style, depth: ColorDepth) {
    buf.extend_from_slice(b"\x1b[0");
    let mods = style.modifiers();
    if mods.contains(Modifiers::BOLD) {
        buf.extend_from_slice(b";1");
    }
    if mods.contains(Modifiers::DIM) {
        buf.extend_from_slice(b";2");
    }
    if mods.contains(Modifiers::ITALIC) {
        buf.extend_from_slice(b";3");
    }
    if mods.contains(Modifiers::UNDERLINED) {
        buf.extend_from_slice(b";4");
    }
    if mods.contains(Modifiers::REVERSED) {
        buf.extend_from_slice(b";7");
    }
    if mods.contains(Modifiers::CROSSED_OUT) {
        buf.extend_from_slice(b";9");
    }

    if depth != ColorDepth::NoColor {
        if let Some(fg) = style.foreground() {
            push_color_sgr(buf, fg, true);
        }
        if let Some(bg) = style.background() {
            push_color_sgr(buf, bg, false);
        }
    }
    buf.push(b'm');
}

fn push_color_sgr(buf: &mut Vec<u8>, color: Color, is_foreground: bool) {
    match color {
        Color::Reset => {}
        Color::Black => buf.extend_from_slice(if is_foreground { b";30" } else { b";40" }),
        Color::Red => buf.extend_from_slice(if is_foreground { b";31" } else { b";41" }),
        Color::Green => buf.extend_from_slice(if is_foreground { b";32" } else { b";42" }),
        Color::Yellow => buf.extend_from_slice(if is_foreground { b";33" } else { b";43" }),
        Color::Blue => buf.extend_from_slice(if is_foreground { b";34" } else { b";44" }),
        Color::Magenta => buf.extend_from_slice(if is_foreground { b";35" } else { b";45" }),
        Color::Cyan => buf.extend_from_slice(if is_foreground { b";36" } else { b";46" }),
        Color::Gray => buf.extend_from_slice(if is_foreground { b";37" } else { b";47" }),
        Color::DarkGray => buf.extend_from_slice(if is_foreground { b";90" } else { b";100" }),
        Color::LightRed => buf.extend_from_slice(if is_foreground { b";91" } else { b";101" }),
        Color::LightGreen => buf.extend_from_slice(if is_foreground { b";92" } else { b";102" }),
        Color::LightYellow => buf.extend_from_slice(if is_foreground { b";93" } else { b";103" }),
        Color::LightBlue => buf.extend_from_slice(if is_foreground { b";94" } else { b";104" }),
        Color::LightMagenta => buf.extend_from_slice(if is_foreground { b";95" } else { b";105" }),
        Color::LightCyan => buf.extend_from_slice(if is_foreground { b";96" } else { b";106" }),
        Color::White => buf.extend_from_slice(if is_foreground { b";97" } else { b";107" }),
        Color::Indexed(idx) => {
            buf.extend_from_slice(if is_foreground { b";38;5;" } else { b";48;5;" });
            push_u16_decimal(buf, u16::from(idx));
        }
        Color::Rgb(r, g, b) => {
            buf.extend_from_slice(if is_foreground { b";38;2;" } else { b";48;2;" });
            push_u16_decimal(buf, u16::from(r));
            buf.push(b';');
            push_u16_decimal(buf, u16::from(g));
            buf.push(b';');
            push_u16_decimal(buf, u16::from(b));
        }
    }
}

/// Writes `value` in base-10 ASCII directly into `buf` with zero heap allocation.
fn push_u16_decimal(buf: &mut Vec<u8>, mut value: u16) {
    if value == 0 {
        buf.push(b'0');
        return;
    }
    let mut digits = [0u8; 5];
    let mut len = 0usize;
    while value > 0 {
        let rem = (value % 10) as u8;
        digits[len] = b'0' + rem;
        len += 1;
        value /= 10;
    }
    while len > 0 {
        len -= 1;
        buf.push(digits[len]);
    }
}
