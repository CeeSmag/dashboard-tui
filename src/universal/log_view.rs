//! Universal scrollable log viewer sub-view (`LogViewSubView`).
//!
//! A project-independent drill-down page that lists application log lines newest
//! first with level colouring and keyboard scrolling. The host keeps its own
//! `Vec<LogEntry>` and exposes it through a plain `fn(&B) -> &[LogEntry]` accessor,
//! so rendering never allocates or copies the log.

use core::cell::Cell;
use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::color::Color;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::palette::ColorPalette;
use crate::core::priority_strip::KeyHint;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::sub_view::{SubView, SubViewResult};
use crate::widgets::panel::TabPanel;
use crate::widgets::scrollbar::SubCellScrollbar;

/// Severity of a [`LogEntry`].
///
/// # Examples
///
/// ```
/// use tui_rs::universal::LogLevel;
///
/// assert!(LogLevel::Error > LogLevel::Info);
/// assert_eq!(LogLevel::Warn.label(), "WARN ");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum LogLevel {
    /// Verbose diagnostic detail.
    Debug,
    /// Normal operational message.
    #[default]
    Info,
    /// Something unexpected that the app recovered from.
    Warn,
    /// A failure that needs attention.
    Error,
}

impl LogLevel {
    /// Returns a fixed-width (5 column) label such as `"INFO "`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO ",
            Self::Warn => "WARN ",
            Self::Error => "ERROR",
        }
    }

    const fn color(self, palette: &ColorPalette) -> Color {
        match self {
            Self::Debug => palette.dim,
            Self::Info => palette.info,
            Self::Warn => palette.warn,
            Self::Error => palette.err,
        }
    }
}

/// One log line: level, short category tag, and message text.
///
/// # Examples
///
/// ```
/// use tui_rs::universal::{LogEntry, LogLevel};
///
/// let entry = LogEntry::new(LogLevel::Warn, "sync", "retrying upload");
/// assert_eq!(entry.level(), LogLevel::Warn);
/// assert_eq!(entry.category(), "sync");
/// assert_eq!(entry.message(), "retrying upload");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LogEntry {
    level: LogLevel,
    category: String,
    message: String,
}

impl LogEntry {
    /// Creates a new [`LogEntry`].
    #[must_use]
    pub fn new(level: LogLevel, category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            category: category.into(),
            message: message.into(),
        }
    }

    /// Returns the severity.
    #[must_use]
    #[inline]
    pub const fn level(&self) -> LogLevel {
        self.level
    }

    /// Returns the category tag (e.g. `"sync"`).
    #[must_use]
    #[inline]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Returns the message text.
    #[must_use]
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Bounded in-memory log the host can write to and show with [`LogViewSubView`].
///
/// # Examples
///
/// ```
/// use tui_rs::universal::{LogBuffer, LogLevel};
///
/// let mut log = LogBuffer::new(2);
/// log.info("sync", "started");
/// log.warn("sync", "slow");
/// log.error("sync", "failed");
/// assert_eq!(log.len(), 2);
/// assert_eq!(log.as_slice()[1].level(), LogLevel::Error);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogBuffer {
    entries: Vec<LogEntry>,
    capacity: usize,
}

impl LogBuffer {
    /// Creates a log keeping at most `capacity` entries (minimum 1).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            entries: Vec::with_capacity(capacity.min(1024)),
            capacity,
        }
    }

    /// Appends an entry, dropping the oldest one when full.
    pub fn push(
        &mut self,
        level: LogLevel,
        category: impl Into<String>,
        message: impl Into<String>,
    ) {
        if self.capacity == 0 {
            self.capacity = 1000;
        }
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(LogEntry::new(level, category, message));
    }

    /// Appends a [`LogLevel::Debug`] entry.
    pub fn debug(&mut self, category: impl Into<String>, message: impl Into<String>) {
        self.push(LogLevel::Debug, category, message);
    }

    /// Appends a [`LogLevel::Info`] entry.
    pub fn info(&mut self, category: impl Into<String>, message: impl Into<String>) {
        self.push(LogLevel::Info, category, message);
    }

    /// Appends a [`LogLevel::Warn`] entry.
    pub fn warn(&mut self, category: impl Into<String>, message: impl Into<String>) {
        self.push(LogLevel::Warn, category, message);
    }

    /// Appends a [`LogLevel::Error`] entry.
    pub fn error(&mut self, category: impl Into<String>, message: impl Into<String>) {
        self.push(LogLevel::Error, category, message);
    }

    /// Returns the entries, oldest first (the shape [`LogViewSubView`] expects).
    #[must_use]
    #[inline]
    pub fn as_slice(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Returns the number of entries.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the log is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns how many entries have the given `level`.
    #[must_use]
    pub fn count_level(&self, level: LogLevel) -> usize {
        self.entries.iter().filter(|e| e.level() == level).count()
    }
}

/// Accessor returning the host's chronological (oldest first) log entries.
pub type LogEntriesFn<B> = fn(&B) -> &[LogEntry];

/// Full-area scrollable log viewer showing the newest entry first.
///
/// Keys: `↑`/`k`, `↓`/`j` scroll a line, `PgUp`/`PgDn` ten lines, `Home`/`End` jump to
/// the newest/oldest entry, `Esc`/`b`/`Enter` go back.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::DashboardBackend;
/// use tui_rs::universal::{LogEntry, LogLevel, LogViewSubView};
///
/// struct App {
///     log: Vec<LogEntry>,
/// }
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let view = LogViewSubView::<App>::new("Live Logs", |app| app.log.as_slice());
/// assert_eq!(view.scroll_offset(), 0);
/// # let _ = LogEntry::new(LogLevel::Info, "boot", "started");
/// ```
pub struct LogViewSubView<B: DashboardBackend> {
    title: String,
    entries: LogEntriesFn<B>,
    scroll_offset: usize,
    visible_rows: Cell<usize>,
    total_rows: Cell<usize>,
}

impl<B: DashboardBackend> core::fmt::Debug for LogViewSubView<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LogViewSubView")
            .field("title", &self.title)
            .field("scroll_offset", &self.scroll_offset)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> LogViewSubView<B> {
    /// Creates a viewer titled `title` that reads its lines through `entries`.
    #[must_use]
    pub fn new(title: impl Into<String>, entries: LogEntriesFn<B>) -> Self {
        Self {
            title: title.into(),
            entries,
            scroll_offset: 0,
            visible_rows: Cell::new(0),
            total_rows: Cell::new(0),
        }
    }

    /// Returns how many lines the view is scrolled down from the newest entry.
    #[must_use]
    #[inline]
    pub const fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    fn max_scroll_offset(&self) -> usize {
        self.total_rows
            .get()
            .saturating_sub(self.visible_rows.get())
    }

    fn scroll_by(&mut self, lines_down: isize) {
        let magnitude = lines_down.unsigned_abs();
        let target = if lines_down.is_negative() {
            self.scroll_offset.saturating_sub(magnitude)
        } else {
            self.scroll_offset.saturating_add(magnitude)
        };
        self.scroll_offset = target.min(self.max_scroll_offset());
    }
}

impl<B: DashboardBackend> SubView<B> for LogViewSubView<B> {
    fn title(&self) -> &str {
        &self.title
    }

    fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        let entries = (self.entries)(backend);
        let mut badge = StackStr::<24>::new();
        let _ = write!(badge, "{} lines", entries.len());

        let inner = TabPanel::new(&self.title)
            .with_badge(badge.as_str())
            .with_subtitle("↑↓ scroll • Esc back")
            .with_focused(true)
            .with_tty_mode(tty_mode)
            .render(buffer, area, palette);
        if inner.is_empty() {
            return;
        }

        let visible_rows = usize::from(inner.height().get());
        self.visible_rows.set(visible_rows);
        self.total_rows.set(entries.len());
        let first_shown = self.scroll_offset.min(self.max_scroll_offset());

        let text_width = inner.width().get().saturating_sub(1);
        let muted = Style::new().with_fg(palette.muted);
        let body = Style::new().with_fg(palette.text);

        if entries.is_empty() {
            let _ = buffer.set_string(inner.x(), inner.y(), "No log lines yet.", muted, text_width);
            return;
        }

        for (screen_row, entry) in entries
            .iter()
            .rev()
            .skip(first_shown)
            .take(visible_rows)
            .enumerate()
        {
            let row_offset = u16::try_from(screen_row).unwrap_or(u16::MAX);
            let row = RowIndex::new(inner.y().get().saturating_add(row_offset));
            let right_edge = inner.x().get().saturating_add(text_width);
            let remaining = |col: ColumnIndex| right_edge.saturating_sub(col.get());

            let level_style = Style::new().with_fg(entry.level().color(palette)).bold();
            let mut col = buffer.set_string(
                inner.x(),
                row,
                entry.level().label(),
                level_style,
                text_width,
            );
            col = buffer.set_string(col, row, " ", muted, remaining(col));
            if !entry.category().is_empty() {
                col = buffer.set_string(col, row, entry.category(), muted, remaining(col));
                col = buffer.set_string(col, row, " ", muted, remaining(col));
            }
            let _ = buffer.set_string(col, row, entry.message(), body, remaining(col));
        }

        SubCellScrollbar::new(entries.len(), visible_rows, first_shown)
            .with_tty_mode(tty_mode)
            .render(
                buffer,
                ColumnIndex::new(inner.right().get().saturating_sub(1)),
                inner.y(),
                inner.height().get(),
                palette,
            );
    }

    fn on_key(&mut self, _backend: &mut B, key: KeyPress) -> SubViewResult {
        const PAGE_LINES: isize = 10;
        match key.code() {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_by(1),
            KeyCode::PageUp => self.scroll_by(-PAGE_LINES),
            KeyCode::PageDown => self.scroll_by(PAGE_LINES),
            KeyCode::Home => self.scroll_offset = 0,
            KeyCode::End => self.scroll_offset = self.max_scroll_offset(),
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('b') => return SubViewResult::Pop,
            _ => {}
        }
        SubViewResult::Continue
    }

    fn footer_hints(&self) -> Vec<KeyHint<'_>> {
        vec![
            KeyHint::new("↑↓", "scroll", Style::DEFAULT, 80),
            KeyHint::new("PgUp/PgDn", "page", Style::DEFAULT, 60),
            KeyHint::new("Home/End", "newest/oldest", Style::DEFAULT, 50),
            KeyHint::new("Esc", "back", Style::DEFAULT, 100),
        ]
    }
}
