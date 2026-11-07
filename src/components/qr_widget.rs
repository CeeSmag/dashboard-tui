//! Dashboard section display widget for QR codes (`QrWidget`).
//!
//! Renders an owned [`QrCodeMatrix`] centered within a [`crate::dashboard::SectionRole::Display`]
//! dashboard section.

use crate::components::qr_code::QrCodeMatrix;
use crate::core::buffer::Buffer;
use crate::core::palette::ColorPalette;
use crate::core::rect::Rect;
use crate::dashboard::{DashboardBackend, SectionWidget};

/// A dashboard section display widget that renders an owned [`QrCodeMatrix`].
///
/// Implements [`SectionWidget`] so it can be passed directly to
/// `.with_display_section(...)` in a dashboard page.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "qrcode")]
/// # {
/// use tui_rs::components::QrWidget;
///
/// let widget = QrWidget::from_text("MOBILE PAIRING", "https://vaultdrive.local/pair").unwrap();
/// assert_eq!(widget.title_str(), "MOBILE PAIRING");
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct QrWidget {
    matrix: QrCodeMatrix,
    title: String,
    subtitle: String,
}

impl QrWidget {
    /// Creates a new [`QrWidget`] with the given title and pre-computed matrix.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{QrCodeMatrix, QrWidget};
    ///
    /// let matrix = QrCodeMatrix::from_raw_modules(vec![true; 4], 2);
    /// let widget = QrWidget::new("STATUS", matrix);
    /// assert_eq!(widget.title_str(), "STATUS");
    /// ```
    #[must_use]
    pub fn new(title: impl Into<String>, matrix: QrCodeMatrix) -> Self {
        Self {
            matrix,
            title: title.into(),
            subtitle: String::new(),
        }
    }

    /// Encodes UTF-8 `text` into a [`QrWidget`] with the given title.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::TuiError::Qr`] if `text` cannot fit in a QR code.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "qrcode")]
    /// # {
    /// use tui_rs::components::QrWidget;
    ///
    /// let widget = QrWidget::from_text("DOCS", "https://github.com").unwrap();
    /// assert_eq!(widget.title_str(), "DOCS");
    /// # }
    /// ```
    #[cfg(feature = "qrcode")]
    pub fn from_text(
        title: impl Into<String>,
        text: impl AsRef<str>,
    ) -> Result<Self, crate::error::TuiError> {
        let matrix = QrCodeMatrix::from_text(text)?;
        Ok(Self::new(title, matrix))
    }

    /// Sets the subtitle displayed in the section's bottom notch.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{QrCodeMatrix, QrWidget};
    ///
    /// let matrix = QrCodeMatrix::from_raw_modules(vec![true; 4], 2);
    /// let widget = QrWidget::new("STATUS", matrix).with_subtitle("Scan to pair");
    /// ```
    #[must_use]
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Returns the title string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{QrCodeMatrix, QrWidget};
    ///
    /// let matrix = QrCodeMatrix::from_raw_modules(vec![true; 4], 2);
    /// let widget = QrWidget::new("PAIR", matrix);
    /// assert_eq!(widget.title_str(), "PAIR");
    /// ```
    #[must_use]
    pub fn title_str(&self) -> &str {
        &self.title
    }

    /// Returns a reference to the underlying [`QrCodeMatrix`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{QrCodeMatrix, QrWidget};
    ///
    /// let matrix = QrCodeMatrix::from_raw_modules(vec![true; 4], 2);
    /// let widget = QrWidget::new("PAIR", matrix);
    /// assert_eq!(widget.matrix().dimension(), 2);
    /// ```
    #[must_use]
    pub const fn matrix(&self) -> &QrCodeMatrix {
        &self.matrix
    }
}

impl<B: DashboardBackend> SectionWidget<B> for QrWidget {
    fn title(&self, _backend: &B) -> String {
        self.title.clone()
    }

    fn subtitle(&self, _backend: &B) -> String {
        self.subtitle.clone()
    }

    fn min_height(&self) -> u16 {
        self.matrix.rendered_height().saturating_add(2)
    }

    fn preferred_height(&self) -> u16 {
        self.matrix.rendered_height().saturating_add(2)
    }

    fn render(
        &self,
        _backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        _palette: &ColorPalette,
        _focused: bool,
        _tty_mode: bool,
    ) {
        let qr_w = self.matrix.rendered_width();
        let qr_h = self.matrix.rendered_height();
        if inner.width().as_u16() < qr_w || inner.height().as_u16() < qr_h {
            return;
        }
        let offset_x = (inner.width().as_u16().saturating_sub(qr_w)) / 2;
        let offset_y = (inner.height().as_u16().saturating_sub(qr_h)) / 2;
        let start_col = inner.x().saturating_add(offset_x);
        let start_row = inner.y().saturating_add(offset_y);
        self.matrix.render(buffer, start_col, start_row);
    }
}
