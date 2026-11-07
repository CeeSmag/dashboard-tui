//! Scrollable multi-line informational / notice dialog (`NoticeModal`).
//!
//! Directly ports `NoticeModal` from `ref/dashboard/addons/universal/prompts.go`,
//! enhanced with a 1/8th sub-cell fractional [`SubCellScrollbar`].

use crate::core::buffer::Buffer;
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::widgets::scrollbar::{SubCellScrollbar, VerticalScrollState};

/// Multi-line scrollable informational modal dialog.
///
/// # Examples
///
/// ```
/// use tui_rs::universal::NoticeModal;
///
/// let modal = NoticeModal::new(
///     "System Diagnostics",
///     vec!["FUSE Mount: OK".to_string(), "rclone Version: v1.68.1".to_string()],
/// );
/// assert_eq!(modal.lines().len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct NoticeModal {
    title: String,
    lines: Vec<String>,
    scroll: VerticalScrollState,
}

impl NoticeModal {
    /// Creates a new [`NoticeModal`] displaying `lines`.
    #[must_use]
    pub fn new(title: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            title: title.into(),
            lines,
            scroll: VerticalScrollState::new(),
        }
    }

    /// Returns the lines displayed in the notice modal.
    #[must_use]
    #[inline]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

impl<B: DashboardBackend> Modal<B> for NoticeModal {
    fn title(&self) -> &str {
        &self.title
    }

    fn subtitle(&self) -> &'static str {
        "↑↓/PgUp/PgDn scroll • Enter/Esc close"
    }

    #[expect(clippy::cast_possible_truncation, reason = "clamped modal dimensions")]
    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let w = (viewport.width().get() * 3 / 4).clamp(44, 78);
        let desired_h = (self.lines.len() as u16).saturating_add(4);
        let max_h = viewport.height().get().saturating_sub(2).max(6);
        let h = desired_h.clamp(6, max_h);
        (TerminalWidth::new(w), TerminalHeight::new(h))
    }

    #[expect(clippy::cast_possible_truncation, reason = "viewport height fits in u16")]
    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let view_h = usize::from(inner.height().get());
        let total = self.lines.len();
        let sb = SubCellScrollbar::new(total, view_h, self.scroll.top()).with_tty_mode(tty_mode);
        let text_w = if sb.should_render() {
            inner.width().get().saturating_sub(2)
        } else {
            inner.width().get()
        };

        let text_style = Style::new().with_fg(palette.text);
        for (row_offset, line) in self
            .lines
            .iter()
            .skip(self.scroll.top())
            .take(view_h)
            .enumerate()
        {
            let ry = inner.y().saturating_add(row_offset as u16);
            let _ = buffer.set_string(inner.x(), ry, line, text_style, text_w);
        }

        if sb.should_render() {
            let sb_col = inner.right().saturating_sub(1);
            sb.render(buffer, sb_col, inner.y(), inner.height().get(), palette);
        }
    }

    fn on_key(&mut self, key: KeyPress, _ctx: &mut ModalContext<'_, B>) -> ModalResult {
        let total = self.lines.len();
        let view_h = 8_usize;
        match key.code() {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => ModalResult::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                let next = self.scroll.top().saturating_sub(1);
                let _ = self.scroll.update(next, total, view_h);
                ModalResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self.scroll.top().saturating_add(1);
                let _ = self.scroll.update(next, total, view_h);
                ModalResult::Continue
            }
            KeyCode::PageUp => {
                let next = self.scroll.top().saturating_sub(view_h);
                let _ = self.scroll.update(next, total, view_h);
                ModalResult::Continue
            }
            KeyCode::PageDown => {
                let next = self.scroll.top().saturating_add(view_h);
                let _ = self.scroll.update(next, total, view_h);
                ModalResult::Continue
            }
            KeyCode::Home => {
                let _ = self.scroll.update(0, total, view_h);
                ModalResult::Continue
            }
            KeyCode::End => {
                let _ = self.scroll.update(total, total, view_h);
                ModalResult::Continue
            }
            _ => ModalResult::Continue,
        }
    }
}
