//! Visual cell styling (`Style` and `Modifiers`).
//!
//! Combines foreground `Color`, background `Color`, and bitflag `Modifiers`
//! (`BOLD`, `DIM`, `ITALIC`, `UNDERLINED`, `REVERSED`, `CROSSED_OUT`).

use bitflags::bitflags;

use crate::core::color::{Color, ColorDepth};

bitflags! {
    /// Bitflag text modifiers corresponding to SGR attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::Modifiers;
    ///
    /// let m = Modifiers::BOLD | Modifiers::UNDERLINED;
    /// assert!(m.contains(Modifiers::BOLD));
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Modifiers: u8 {
        /// SGR 1: Bold / increased intensity.
        const BOLD        = 0b0000_0001;
        /// SGR 2: Dim / faint intensity.
        const DIM         = 0b0000_0010;
        /// SGR 3: Italicized text.
        const ITALIC      = 0b0000_0100;
        /// SGR 4: Underlined text.
        const UNDERLINED  = 0b0000_1000;
        /// SGR 7: Reverse video (swap foreground and background).
        const REVERSED    = 0b0001_0000;
        /// SGR 9: Strikethrough / crossed-out text.
        const CROSSED_OUT = 0b0010_0000;
    }
}

/// Complete visual attribute specification for a [`crate::core::Cell`], [`crate::core::Span`], or widget.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, Modifiers, Style};
///
/// let style = Style::new().fg(Color::Cyan).bold();
/// assert_eq!(style.foreground(), Some(Color::Cyan));
/// assert!(style.modifiers().contains(Modifiers::BOLD));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Style {
    foreground: Option<Color>,
    background: Option<Color>,
    add_modifiers: Modifiers,
    sub_modifiers: Modifiers,
}

impl Style {
    /// Default unstyled configuration.
    pub const DEFAULT: Self = Self {
        foreground: None,
        background: None,
        add_modifiers: Modifiers::empty(),
        sub_modifiers: Modifiers::empty(),
    };

    /// Creates a new default `Style`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::Style;
    ///
    /// assert_eq!(Style::new(), Style::DEFAULT);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// Sets the foreground color.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, Style};
    ///
    /// let s = Style::new().fg(Color::Green);
    /// assert_eq!(s.foreground(), Some(Color::Green));
    /// ```
    #[must_use]
    pub const fn fg(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Builder alias for [`Self::fg`].
    #[must_use]
    #[inline]
    pub const fn with_fg(self, color: Color) -> Self {
        self.fg(color)
    }

    /// Sets the background color.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, Style};
    ///
    /// let s = Style::new().bg(Color::Black);
    /// assert_eq!(s.background(), Some(Color::Black));
    /// ```
    #[must_use]
    pub const fn bg(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Builder alias for [`Self::bg`].
    #[must_use]
    #[inline]
    pub const fn with_bg(self, color: Color) -> Self {
        self.bg(color)
    }

    /// Adds the `BOLD` modifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// assert!(Style::new().bold().modifiers().contains(Modifiers::BOLD));
    /// ```
    #[must_use]
    pub const fn bold(self) -> Self {
        self.add_modifier(Modifiers::BOLD)
    }

    /// Adds the `DIM` modifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// assert!(Style::new().dim().modifiers().contains(Modifiers::DIM));
    /// ```
    #[must_use]
    pub const fn dim(self) -> Self {
        self.add_modifier(Modifiers::DIM)
    }

    /// Adds the `ITALIC` modifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// assert!(Style::new().italic().modifiers().contains(Modifiers::ITALIC));
    /// ```
    #[must_use]
    pub const fn italic(self) -> Self {
        self.add_modifier(Modifiers::ITALIC)
    }

    /// Adds the `UNDERLINED` modifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// assert!(Style::new().underlined().modifiers().contains(Modifiers::UNDERLINED));
    /// ```
    #[must_use]
    pub const fn underlined(self) -> Self {
        self.add_modifier(Modifiers::UNDERLINED)
    }

    /// Adds the `REVERSED` modifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// assert!(Style::new().reversed().modifiers().contains(Modifiers::REVERSED));
    /// ```
    #[must_use]
    pub const fn reversed(self) -> Self {
        self.add_modifier(Modifiers::REVERSED)
    }

    /// Enables one or more [`Modifiers`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// let s = Style::new().add_modifier(Modifiers::BOLD | Modifiers::ITALIC);
    /// assert!(s.modifiers().contains(Modifiers::BOLD));
    /// ```
    #[must_use]
    pub const fn add_modifier(mut self, modifier: Modifiers) -> Self {
        self.sub_modifiers = self.sub_modifiers.difference(modifier);
        self.add_modifiers = self.add_modifiers.union(modifier);
        self
    }

    /// Removes one or more [`Modifiers`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    ///
    /// let s = Style::new().bold().remove_modifier(Modifiers::BOLD);
    /// assert!(!s.modifiers().contains(Modifiers::BOLD));
    /// ```
    #[must_use]
    pub const fn remove_modifier(mut self, modifier: Modifiers) -> Self {
        self.add_modifiers = self.add_modifiers.difference(modifier);
        self.sub_modifiers = self.sub_modifiers.union(modifier);
        self
    }

    /// Overlays `other` on top of `self`, preserving `self` attributes where `other` is unset.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, Style};
    ///
    /// let base = Style::new().fg(Color::White).bg(Color::Black);
    /// let accent = Style::new().fg(Color::Cyan).bold();
    /// let merged = base.patch(accent);
    /// assert_eq!(merged.foreground(), Some(Color::Cyan));
    /// assert_eq!(merged.background(), Some(Color::Black));
    /// ```
    #[must_use]
    pub const fn patch(mut self, other: Self) -> Self {
        if other.foreground.is_some() {
            self.foreground = other.foreground;
        }
        if other.background.is_some() {
            self.background = other.background;
        }
        self.add_modifiers = self
            .add_modifiers
            .difference(other.sub_modifiers)
            .union(other.add_modifiers);
        self.sub_modifiers = self
            .sub_modifiers
            .difference(other.add_modifiers)
            .union(other.sub_modifiers);
        self
    }

    /// Returns the configured foreground [`Color`], if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, Style};
    /// assert_eq!(Style::new().fg(Color::Red).foreground(), Some(Color::Red));
    /// ```
    #[must_use]
    pub const fn foreground(self) -> Option<Color> {
        self.foreground
    }

    /// Returns the configured background [`Color`], if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, Style};
    /// assert_eq!(Style::new().bg(Color::Blue).background(), Some(Color::Blue));
    /// ```
    #[must_use]
    pub const fn background(self) -> Option<Color> {
        self.background
    }

    /// Returns the active [`Modifiers`] bitflags.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Modifiers, Style};
    /// assert_eq!(Style::new().bold().modifiers(), Modifiers::BOLD);
    /// ```
    #[must_use]
    pub const fn modifiers(self) -> Modifiers {
        self.add_modifiers
    }

    /// Degrades foreground and background colors to the target [`ColorDepth`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, ColorDepth, Style};
    ///
    /// let s = Style::new().fg(Color::Rgb(255, 0, 0)).bold();
    /// let no_color = s.degrade(ColorDepth::NoColor);
    /// assert_eq!(no_color.foreground(), Some(Color::Reset));
    /// ```
    #[must_use]
    pub const fn degrade(mut self, depth: ColorDepth) -> Self {
        if let Some(fg) = self.foreground {
            self.foreground = Some(fg.degrade(depth));
        }
        if let Some(bg) = self.background {
            self.background = Some(bg.degrade(depth));
        }
        self
    }
}
