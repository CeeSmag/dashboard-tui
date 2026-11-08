//! Fluent styling (`Stylize`): `"text".red().bold()`, `Span::raw("x").on_blue()`.
//!
//! The trait is implemented for [`Style`] itself (chaining), [`Span`], `&str` and `String`
//! (both producing a [`Span`]), so a styled run of text needs no builder boilerplate.

use crate::core::color::Color;
use crate::core::style::Style;
use crate::core::text::Span;

macro_rules! foreground_colors {
    ($($method:ident => $color:expr),* $(,)?) => {
        $(
            /// Sets the foreground color.
            #[must_use]
            fn $method(self) -> Self::Styled {
                self.patch_style(Style::new().fg($color))
            }
        )*
    };
}

macro_rules! background_colors {
    ($($method:ident => $color:expr),* $(,)?) => {
        $(
            /// Sets the background color.
            #[must_use]
            fn $method(self) -> Self::Styled {
                self.patch_style(Style::new().bg($color))
            }
        )*
    };
}

/// Fluent style shortcuts for text and styles.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, Span, Style, Stylize};
///
/// let span: Span<'_> = "ERROR".red().bold();
/// assert_eq!(span.style(), Style::new().fg(Color::Red).bold());
///
/// let banner = "ready".green().on_black().italic();
/// assert_eq!(banner.style().background(), Some(Color::Black));
///
/// // Works on `Style` too, for chained palettes:
/// assert_eq!(Style::new().cyan().underlined(), Style::new().fg(Color::Cyan).underlined());
/// ```
pub trait Stylize: Sized {
    /// What styling produces (`Style` for styles, `Span` for text).
    type Styled;

    /// Layers `style` on top of the current style.
    #[must_use]
    fn patch_style(self, style: Style) -> Self::Styled;

    /// Sets an arbitrary foreground color.
    #[must_use]
    fn fg_color(self, color: Color) -> Self::Styled {
        self.patch_style(Style::new().fg(color))
    }

    /// Sets an arbitrary background color.
    #[must_use]
    fn bg_color(self, color: Color) -> Self::Styled {
        self.patch_style(Style::new().bg(color))
    }

    foreground_colors! {
        black => Color::Black,
        red => Color::Red,
        green => Color::Green,
        yellow => Color::Yellow,
        blue => Color::Blue,
        magenta => Color::Magenta,
        cyan => Color::Cyan,
        gray => Color::Gray,
        dark_gray => Color::DarkGray,
        light_red => Color::LightRed,
        light_green => Color::LightGreen,
        light_yellow => Color::LightYellow,
        light_blue => Color::LightBlue,
        light_magenta => Color::LightMagenta,
        light_cyan => Color::LightCyan,
        white => Color::White,
    }

    background_colors! {
        on_black => Color::Black,
        on_red => Color::Red,
        on_green => Color::Green,
        on_yellow => Color::Yellow,
        on_blue => Color::Blue,
        on_magenta => Color::Magenta,
        on_cyan => Color::Cyan,
        on_gray => Color::Gray,
        on_dark_gray => Color::DarkGray,
        on_white => Color::White,
    }

    /// Adds the bold modifier.
    #[must_use]
    fn bold(self) -> Self::Styled {
        self.patch_style(Style::new().bold())
    }

    /// Adds the dim modifier.
    #[must_use]
    fn dim(self) -> Self::Styled {
        self.patch_style(Style::new().dim())
    }

    /// Adds the italic modifier.
    #[must_use]
    fn italic(self) -> Self::Styled {
        self.patch_style(Style::new().italic())
    }

    /// Adds the underline modifier.
    #[must_use]
    fn underlined(self) -> Self::Styled {
        self.patch_style(Style::new().underlined())
    }

    /// Adds the reversed (swap fg/bg) modifier.
    #[must_use]
    fn reversed(self) -> Self::Styled {
        self.patch_style(Style::new().reversed())
    }
}

impl Stylize for Style {
    type Styled = Self;

    fn patch_style(self, style: Style) -> Self {
        self.patch(style)
    }
}

impl Stylize for Span<'_> {
    type Styled = Self;

    fn patch_style(self, style: Style) -> Self {
        Span::patch_style(self, style)
    }
}

impl<'a> Stylize for &'a str {
    type Styled = Span<'a>;

    fn patch_style(self, style: Style) -> Span<'a> {
        Span::styled(self, style)
    }
}

impl Stylize for String {
    type Styled = Span<'static>;

    fn patch_style(self, style: Style) -> Span<'static> {
        Span::styled(self, style)
    }
}
