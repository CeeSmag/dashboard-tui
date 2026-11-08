//! Styled inline text primitives (`Span`, `Line`, and `TextAlignment`).
//!
//! Designed for zero-copy borrowing (`Cow<'a, str>`) so static strings and borrowed
//! telemetry slices never allocate on the heap during rendering.

use std::borrow::Cow;

use crate::core::coordinates::TerminalWidth;
use crate::core::grapheme::display_width;
use crate::core::style::Style;

/// Horizontal alignment of a [`Line`] or text block within a [`crate::core::Rect`].
///
/// # Examples
///
/// ```
/// use tui_rs::core::TextAlignment;
///
/// assert_eq!(TextAlignment::default(), TextAlignment::Left);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextAlignment {
    /// Align text flush with the left edge.
    #[default]
    Left,
    /// Center text horizontally within the available width.
    Center,
    /// Align text flush with the right edge.
    Right,
}

/// A contiguous inline run of text sharing a single [`Style`] with no line breaks.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, Span, Style, TerminalWidth};
///
/// let span = Span::styled("● MOUNTED", Style::new().fg(Color::Green).bold());
/// assert_eq!(span.width(), TerminalWidth::new(9));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span<'a> {
    content: Cow<'a, str>,
    style: Style,
    hyperlink: Option<Cow<'a, str>>,
}

impl<'a> Span<'a> {
    /// Creates an unstyled `Span` from a string or borrowed slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Span;
    ///
    /// let s = Span::raw("VaultDrive");
    /// assert_eq!(s.as_str(), "VaultDrive");
    /// ```
    #[must_use]
    pub fn raw(content: impl Into<Cow<'a, str>>) -> Self {
        Self {
            content: content.into(),
            style: Style::DEFAULT,
            hyperlink: None,
        }
    }

    /// Creates a styled `Span` with the given [`Style`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, Span, Style};
    ///
    /// let s = Span::styled("Online", Style::new().fg(Color::Green));
    /// assert_eq!(s.style().foreground(), Some(Color::Green));
    /// ```
    #[must_use]
    pub fn styled(content: impl Into<Cow<'a, str>>, style: Style) -> Self {
        Self {
            content: content.into(),
            style,
            hyperlink: None,
        }
    }

    /// Creates a clickable hyperlink [`Span`] with the given text and target URL.
    ///
    /// The span defaults to an underlined cyan visual style (`Modifiers::UNDERLINED` and `Color::Cyan`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Span;
    ///
    /// let link = Span::hyperlink("VaultDrive Home", "https://github.com/vaultdrive");
    /// assert_eq!(link.as_str(), "VaultDrive Home");
    /// assert_eq!(link.hyperlink_url(), Some("https://github.com/vaultdrive"));
    /// ```
    #[must_use]
    pub fn hyperlink(content: impl Into<Cow<'a, str>>, url: impl Into<Cow<'a, str>>) -> Self {
        Self {
            content: content.into(),
            style: Style::new().fg(crate::core::Color::Cyan).underlined(),
            hyperlink: Some(url.into()),
        }
    }

    /// Attaches or clears the target OSC 8 hyperlink URL on this [`Span`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Span;
    ///
    /// let span = Span::raw("GitHub").with_hyperlink("https://github.com");
    /// assert_eq!(span.hyperlink_url(), Some("https://github.com"));
    /// ```
    #[must_use]
    pub fn with_hyperlink(mut self, url: impl Into<Cow<'a, str>>) -> Self {
        self.hyperlink = Some(url.into());
        self
    }

    /// Returns the target OSC 8 hyperlink URL attached to this span, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Span;
    ///
    /// let span = Span::raw("Plain");
    /// assert_eq!(span.hyperlink_url(), None);
    ///
    /// let link = Span::hyperlink("GitHub", "https://github.com");
    /// assert_eq!(link.hyperlink_url(), Some("https://github.com"));
    /// ```
    #[must_use]
    pub fn hyperlink_url(&self) -> Option<&str> {
        self.hyperlink.as_deref()
    }

    /// Returns a borrowed view of the span's text content (`C-CONV` `as_str`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Span;
    /// assert_eq!(Span::raw("abc").as_str(), "abc");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.content
    }

    /// Returns the visual [`Style`] of this span.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Span, Style};
    /// assert_eq!(Span::raw("x").style(), Style::DEFAULT);
    /// ```
    #[must_use]
    pub const fn style(&self) -> Style {
        self.style
    }

    /// Computes the visual column width of this span using UAX #29 grapheme widths.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Span, TerminalWidth};
    /// assert_eq!(Span::raw("100%").width(), TerminalWidth(4));
    /// ```
    #[must_use]
    pub fn width(&self) -> TerminalWidth {
        display_width(&self.content)
    }

    /// Returns true if the span contains no characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Span;
    /// assert!(Span::raw("").is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(value: &'a str) -> Self {
        Self::raw(value)
    }
}

impl From<String> for Span<'static> {
    fn from(value: String) -> Self {
        Self::raw(value)
    }
}

/// An ordered horizontal sequence of [`Span`] elements representing a single row of styled text.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, Line, Span, Style, TerminalWidth};
///
/// let line = Line::from_spans([
///     Span::styled("Route: ", Style::new().dim()),
///     Span::styled("DIRECT_LAN", Style::new().fg(Color::Cyan).bold()),
/// ]);
/// assert_eq!(line.width(), TerminalWidth::new(17));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Line<'a> {
    spans: Vec<Span<'a>>,
    style: Style,
    alignment: TextAlignment,
}

impl<'a> Line<'a> {
    /// Creates an empty `Line`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Line;
    /// assert!(Line::empty().is_empty());
    /// ```
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            spans: Vec::new(),
            style: Style::DEFAULT,
            alignment: TextAlignment::Left,
        }
    }

    /// Creates a `Line` from an iterator of [`Span`] items.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Line, Span};
    ///
    /// let line = Line::from_spans([Span::raw("A"), Span::raw("B")]);
    /// assert_eq!(line.as_spans().len(), 2);
    /// ```
    #[must_use]
    pub fn from_spans(spans: impl IntoIterator<Item = Span<'a>>) -> Self {
        Self {
            spans: spans.into_iter().collect(),
            style: Style::DEFAULT,
            alignment: TextAlignment::Left,
        }
    }

    /// Creates a single-span unstyled `Line`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Line;
    /// let line = Line::raw("Hello");
    /// assert_eq!(line.width().as_u16(), 5);
    /// ```
    #[must_use]
    pub fn raw(content: impl Into<Cow<'a, str>>) -> Self {
        Self {
            spans: vec![Span::raw(content)],
            style: Style::DEFAULT,
            alignment: TextAlignment::Left,
        }
    }

    /// Creates a single-span styled `Line`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, Line, Style};
    /// let line = Line::styled("Header", Style::new().fg(Color::Cyan));
    /// assert_eq!(line.as_spans().len(), 1);
    /// ```
    #[must_use]
    pub fn styled(content: impl Into<Cow<'a, str>>, style: Style) -> Self {
        Self {
            spans: vec![Span::styled(content, style)],
            style: Style::DEFAULT,
            alignment: TextAlignment::Left,
        }
    }

    /// Sets the horizontal [`TextAlignment`] of this line.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Line, TextAlignment};
    /// let line = Line::raw("Centered").alignment(TextAlignment::Center);
    /// assert_eq!(line.text_alignment(), TextAlignment::Center);
    /// ```
    #[must_use]
    pub const fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the base [`Style`] applied underneath all spans in this line.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, Line, Style};
    /// let line = Line::raw("Row").style(Style::new().bg(Color::Black));
    /// assert_eq!(line.base_style().background(), Some(Color::Black));
    /// ```
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns a borrowed slice of the spans in this line (`C-CONV` `as_spans`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Line;
    /// assert!(Line::empty().as_spans().is_empty());
    /// ```
    #[must_use]
    pub fn as_spans(&self) -> &[Span<'a>] {
        &self.spans
    }

    /// Returns the base [`Style`] of the line.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Line, Style};
    /// assert_eq!(Line::empty().base_style(), Style::DEFAULT);
    /// ```
    #[must_use]
    pub const fn base_style(&self) -> Style {
        self.style
    }

    /// Returns the horizontal [`TextAlignment`] of the line.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Line, TextAlignment};
    /// assert_eq!(Line::empty().text_alignment(), TextAlignment::Left);
    /// ```
    #[must_use]
    pub const fn text_alignment(&self) -> TextAlignment {
        self.alignment
    }

    /// Computes the total visual column width across all spans in this line.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Line, Span, TerminalWidth};
    /// let l = Line::from_spans([Span::raw("ab"), Span::raw("cde")]);
    /// assert_eq!(l.width(), TerminalWidth(5));
    /// ```
    #[must_use]
    pub fn width(&self) -> TerminalWidth {
        let mut total = TerminalWidth::ZERO;
        for span in &self.spans {
            total = total.saturating_add(span.width().as_u16());
        }
        total
    }

    /// Returns true if the line has no spans or all spans are empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Line;
    /// assert!(Line::raw("").is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(Span::is_empty)
    }
}

impl<'a> From<&'a str> for Line<'a> {
    fn from(value: &'a str) -> Self {
        Self::raw(value)
    }
}

impl From<String> for Line<'static> {
    fn from(value: String) -> Self {
        Self::raw(value)
    }
}

impl<'a> From<Span<'a>> for Line<'a> {
    fn from(value: Span<'a>) -> Self {
        Self::from_spans([value])
    }
}

/// An ordered vertical sequence of [`Line`]s (a multi-line block of styled text).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Line, Text};
///
/// let text = Text::from("first\nsecond line");
/// assert_eq!(text.height(), 2);
/// assert_eq!(text.width().get(), 11);
/// let built = Text::from_lines([Line::raw("a"), Line::raw("bb")]);
/// assert_eq!(built.lines().len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Text<'a> {
    lines: Vec<Line<'a>>,
}

impl<'a> Text<'a> {
    /// Creates an empty [`Text`].
    #[must_use]
    pub const fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Builds a [`Text`] from lines.
    #[must_use]
    pub fn from_lines(lines: impl IntoIterator<Item = Line<'a>>) -> Self {
        Self {
            lines: lines.into_iter().collect(),
        }
    }

    /// Appends a line.
    pub fn push_line(&mut self, line: impl Into<Line<'a>>) {
        self.lines.push(line.into());
    }

    /// Returns the lines.
    #[must_use]
    pub fn lines(&self) -> &[Line<'a>] {
        &self.lines
    }

    /// Returns the number of lines.
    #[must_use]
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Returns the width of the widest line.
    #[must_use]
    pub fn width(&self) -> TerminalWidth {
        self.lines
            .iter()
            .map(Line::width)
            .max()
            .unwrap_or(TerminalWidth::new(0))
    }

    /// Returns `true` if there are no lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            lines: value.split('\n').map(Line::raw).collect(),
        }
    }
}

impl From<String> for Text<'static> {
    fn from(value: String) -> Self {
        Self {
            lines: value
                .split('\n')
                .map(|part| Line::raw(part.to_string()))
                .collect(),
        }
    }
}

impl<'a> From<Line<'a>> for Text<'a> {
    fn from(value: Line<'a>) -> Self {
        Self { lines: vec![value] }
    }
}

impl<'a> From<Vec<Line<'a>>> for Text<'a> {
    fn from(value: Vec<Line<'a>>) -> Self {
        Self { lines: value }
    }
}
