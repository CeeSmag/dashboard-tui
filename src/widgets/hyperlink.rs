//! Clickable terminal hyperlink widget (`Hyperlink`).
//!
//! Emits OSC 8 escape sequences allowing modern terminal emulators (iTerm2, Alacritty,
//! Kitty, `WezTerm`, VS Code, Windows Terminal) to treat text as clickable web links.

use std::borrow::Cow;

use crate::core::buffer::Buffer;
use crate::core::color::Color;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::display_width;
use crate::core::style::Style;
use crate::core::text::Span;

/// A clickable terminal hyperlink widget rendered using OSC 8 escape sequences.
///
/// Clicking the rendered anchor text in supporting terminal emulators opens the target
/// URL in the user's default web browser. In terminals without OSC 8 support, the anchor
/// text is rendered normally with its configured style (underlined cyan by default).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
/// use dashboard_tui::widgets::Hyperlink;
///
/// let area = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(1));
/// let mut buf = Buffer::empty(area);
/// let link = Hyperlink::new("https://github.com/CeeSmag/dashboard-tui", "VaultDrive Docs");
/// link.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 30);
/// assert!(buf.to_visual_lines()[0].contains("VaultDrive Docs"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink<'a> {
    url: Cow<'a, str>,
    text: Cow<'a, str>,
    style: Style,
}

impl<'a> Hyperlink<'a> {
    /// Creates a new [`Hyperlink`] targeting `url` with visible anchor `text`.
    ///
    /// Defaults to an underlined cyan visual style (`Color::Cyan` and `Modifiers::UNDERLINED`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://github.com", "GitHub");
    /// assert_eq!(link.url(), "https://github.com");
    /// assert_eq!(link.text(), "GitHub");
    /// ```
    #[must_use]
    pub fn new(url: impl Into<Cow<'a, str>>, text: impl Into<Cow<'a, str>>) -> Self {
        Self {
            url: url.into(),
            text: text.into(),
            style: Style::new().fg(Color::Cyan).underlined(),
        }
    }

    /// Sets a custom visual [`Style`] for the link's anchor text.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, Style};
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://example.com", "Example")
    ///     .with_style(Style::new().fg(Color::Green).bold());
    /// assert_eq!(link.style().foreground(), Some(Color::Green));
    /// ```
    #[must_use]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns the target URL of this hyperlink.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://github.com", "GitHub");
    /// assert_eq!(link.url(), "https://github.com");
    /// ```
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the visible anchor text of this hyperlink.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://github.com", "GitHub");
    /// assert_eq!(link.text(), "GitHub");
    /// ```
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the visual [`Style`] of this hyperlink.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://github.com", "GitHub");
    /// assert!(link.style().modifiers().contains(dashboard_tui::core::Modifiers::UNDERLINED));
    /// ```
    #[must_use]
    pub const fn style(&self) -> Style {
        self.style
    }

    /// Computes the visual column width of this hyperlink's anchor text.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TerminalWidth;
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://github.com", "Docs");
    /// assert_eq!(link.width(), TerminalWidth::new(4));
    /// ```
    #[must_use]
    pub fn width(&self) -> TerminalWidth {
        display_width(&self.text)
    }

    /// Converts this hyperlink into a styled [`Span`] with the attached target URL.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::Span;
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let link = Hyperlink::new("https://github.com", "GitHub");
    /// let span: Span<'_> = link.to_span();
    /// assert_eq!(span.hyperlink_url(), Some("https://github.com"));
    /// ```
    #[must_use]
    pub fn to_span(&self) -> Span<'a> {
        Span::styled(self.text.clone(), self.style).with_hyperlink(self.url.clone())
    }

    /// Renders the hyperlink into `buffer` at `(column, row)` clamped to `max_width` columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Buffer, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::widgets::Hyperlink;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(20), TerminalHeight::new(1));
    /// let mut buf = Buffer::empty(area);
    /// let link = Hyperlink::new("https://rust-lang.org", "Rust");
    /// link.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 20);
    /// ```
    pub fn render(
        &self,
        buffer: &mut Buffer,
        column: ColumnIndex,
        row: RowIndex,
        max_width: u16,
    ) -> ColumnIndex {
        buffer.set_hyperlink(column, row, &self.url, &self.text, self.style, max_width)
    }
}

impl<'a> From<Hyperlink<'a>> for Span<'a> {
    fn from(link: Hyperlink<'a>) -> Self {
        Span::styled(link.text, link.style).with_hyperlink(link.url)
    }
}
