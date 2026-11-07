//! Scrollable multi-line informational / notice dialog (`NoticeModal`).
//!
//! Directly ports `NoticeModal` from `ref/dashboard/addons/universal/prompts.go`,
//! enhanced with a 1/8th sub-cell fractional [`SubCellScrollbar`] and optional
//! embedded [`QrCodeMatrix`] for mobile companion pairing or documentation links.

use crate::components::qr_code::QrCodeMatrix;
use crate::core::buffer::Buffer;
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::widgets::scrollbar::{SubCellScrollbar, VerticalScrollState};

/// Multi-line scrollable informational modal dialog with optional embedded QR code.
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
    qr_matrix: Option<QrCodeMatrix>,
}

impl NoticeModal {
    /// Creates a new [`NoticeModal`] displaying `lines`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::universal::NoticeModal;
    ///
    /// let modal = NoticeModal::new("Notice", vec!["Line 1".to_string()]);
    /// assert_eq!(modal.lines().len(), 1);
    /// ```
    #[must_use]
    pub fn new(title: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            title: title.into(),
            lines,
            scroll: VerticalScrollState::new(),
            qr_matrix: None,
        }
    }

    /// Attaches an optional pre-computed [`QrCodeMatrix`] to be displayed centered below the text lines.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::QrCodeMatrix;
    /// use tui_rs::universal::NoticeModal;
    ///
    /// let matrix = QrCodeMatrix::from_raw_modules(vec![true; 4], 2);
    /// let modal = NoticeModal::new("QR Notice", vec!["Scan below".to_string()])
    ///     .with_qr_matrix(matrix);
    /// assert!(modal.qr_matrix().is_some());
    /// ```
    #[must_use]
    pub fn with_qr_matrix(mut self, matrix: QrCodeMatrix) -> Self {
        self.qr_matrix = Some(matrix);
        self
    }

    /// Generates and attaches a [`QrCodeMatrix`] encoding `text` to be displayed centered below the lines.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::TuiError::Qr`] if `text` exceeds QR code capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "qrcode")]
    /// # {
    /// use tui_rs::universal::NoticeModal;
    ///
    /// let modal = NoticeModal::new("Documentation", vec!["Repo:".to_string()])
    ///     .with_qr_text("https://github.com/vaultdrive/tui-rs")
    ///     .unwrap();
    /// assert!(modal.qr_matrix().is_some());
    /// # }
    /// ```
    #[cfg(feature = "qrcode")]
    pub fn with_qr_text(mut self, text: impl AsRef<str>) -> Result<Self, crate::error::TuiError> {
        let matrix = QrCodeMatrix::from_text(text)?.with_quiet_zone(1);
        self.qr_matrix = Some(matrix);
        Ok(self)
    }

    /// Returns the lines displayed in the notice modal.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::universal::NoticeModal;
    ///
    /// let modal = NoticeModal::new("Title", vec!["Hello".to_string()]);
    /// assert_eq!(modal.lines()[0], "Hello");
    /// ```
    #[must_use]
    #[inline]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Returns a reference to the embedded QR code matrix, if configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::universal::NoticeModal;
    ///
    /// let modal = NoticeModal::new("Title", vec![]);
    /// assert!(modal.qr_matrix().is_none());
    /// ```
    #[must_use]
    #[inline]
    pub const fn qr_matrix(&self) -> Option<&QrCodeMatrix> {
        self.qr_matrix.as_ref()
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
        let mut min_w = 44_u16;
        let mut extra_h = 0_u16;
        if let Some(qr) = &self.qr_matrix {
            min_w = min_w.max(qr.rendered_width().saturating_add(6));
            extra_h = qr.rendered_height().saturating_add(1);
        }
        let w = (viewport.width().get() * 4 / 5).clamp(min_w, 80);
        let desired_h = (self.lines.len() as u16)
            .saturating_add(4)
            .saturating_add(extra_h);
        let max_h = viewport.height().get().saturating_sub(2).max(6);
        let h = desired_h.clamp(6, max_h);
        (TerminalWidth::new(w), TerminalHeight::new(h))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "viewport height fits in u16"
    )]
    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let view_h = usize::from(inner.height().get());
        let qr_h = self
            .qr_matrix
            .as_ref()
            .map_or(0, |q| usize::from(q.rendered_height()).saturating_add(1));
        let text_avail_h = view_h.saturating_sub(qr_h);

        let total = self.lines.len();
        let sb =
            SubCellScrollbar::new(total, text_avail_h, self.scroll.top()).with_tty_mode(tty_mode);
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
            .take(text_avail_h)
            .enumerate()
        {
            let ry = inner.y().saturating_add(row_offset as u16);
            let trimmed = line.trim();
            if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
                let link_style = Style::new().with_fg(palette.accent).underlined();
                let _ = buffer.set_hyperlink(inner.x(), ry, trimmed, line, link_style, text_w);
            } else {
                let _ = buffer.set_string(inner.x(), ry, line, text_style, text_w);
            }
        }

        if sb.should_render() {
            let sb_col = inner.right().saturating_sub(1);
            sb.render(buffer, sb_col, inner.y(), text_avail_h as u16, palette);
        }

        if let Some(qr) = &self.qr_matrix {
            let qr_w = qr.rendered_width();
            let qr_rendered_h = qr.rendered_height();
            if inner.height().get() >= qr_rendered_h && inner.width().get() >= qr_w {
                let lines_rendered =
                    total.saturating_sub(self.scroll.top()).min(text_avail_h) as u16;
                let preferred_y = inner.y().saturating_add(lines_rendered).saturating_add(1);
                let max_start_y = inner.bottom().saturating_sub(qr_rendered_h);
                let qr_y = preferred_y.min(max_start_y);
                let offset_x = (inner.width().get().saturating_sub(qr_w)) / 2;
                let start_x = inner.x().saturating_add(offset_x);
                qr.render(buffer, start_x, qr_y);
            }
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
