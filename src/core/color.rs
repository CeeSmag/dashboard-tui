//! Terminal color primitives and automatic capability downsampling.
//!
//! Supports 24-bit `TrueColor` (`Color::Rgb`), 256-color (`Color::Indexed`),
//! 16-color standard ANSI, and `NO_COLOR` / dumb terminal degradation.

use crate::error::TuiError;

/// Color capability level supported by the target terminal.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, ColorDepth};
///
/// let rgb = Color::Rgb(56, 189, 248);
/// assert_eq!(rgb.degrade(ColorDepth::NoColor), Color::Reset);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColorDepth {
    /// Suppresses all color escape codes (`NO_COLOR` or dumb terminal).
    NoColor,
    /// Standard 16-color ANSI palette (Linux virtual console `/dev/tty*`, `TERM=linux`).
    Ansi16,
    /// 8-bit 256-color xterm palette (`TERM=*-256color`).
    Ansi256,
    /// 24-bit `TrueColor` RGB (`COLORTERM=truecolor|24bit`).
    #[default]
    TrueColor,
}

/// Terminal foreground, background, or underline color.
///
/// # Examples
///
/// ```
/// use tui_rs::core::Color;
///
/// let cyan = Color::from_hex("#38BDF8").unwrap();
/// assert_eq!(cyan, Color::Rgb(56, 189, 248));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Color {
    /// Resets the color attribute to the terminal's default.
    #[default]
    Reset,
    /// Standard ANSI Black (index 0).
    Black,
    /// Standard ANSI Red (index 1).
    Red,
    /// Standard ANSI Green (index 2).
    Green,
    /// Standard ANSI Yellow (index 3).
    Yellow,
    /// Standard ANSI Blue (index 4).
    Blue,
    /// Standard ANSI Magenta (index 5).
    Magenta,
    /// Standard ANSI Cyan (index 6).
    Cyan,
    /// Standard ANSI Gray / White (index 7).
    Gray,
    /// Bright ANSI Black / Dark Gray (index 8).
    DarkGray,
    /// Bright ANSI Red (index 9).
    LightRed,
    /// Bright ANSI Green (index 10).
    LightGreen,
    /// Bright ANSI Yellow (index 11).
    LightYellow,
    /// Bright ANSI Blue (index 12).
    LightBlue,
    /// Bright ANSI Magenta (index 13).
    LightMagenta,
    /// Bright ANSI Cyan (index 14).
    LightCyan,
    /// Bright ANSI White (index 15).
    White,
    /// 8-bit 256-color palette index (`0..=255`).
    Indexed(u8),
    /// 24-bit `TrueColor` `(red, green, blue)`.
    Rgb(u8, u8, u8),
}

impl Color {
    /// Parses a `#RRGGBB` or `RRGGBB` hex string (or an ANSI index `"0"`..`"15"`) into a [`Color`].
    ///
    /// # Errors
    ///
    /// Returns [`TuiError::InvalidHexColor`] if the string is neither a valid 6-digit hex code
    /// nor a valid `u8` ANSI index.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Color;
    ///
    /// assert_eq!(Color::from_hex("#00B4D8").unwrap(), Color::Rgb(0, 180, 216));
    /// assert_eq!(Color::from_hex("14").unwrap(), Color::LightCyan);
    /// ```
    pub fn from_hex(input: impl AsRef<str>) -> Result<Self, TuiError> {
        let trimmed = input.as_ref().trim();
        if let Ok(idx) = trimmed.parse::<u8>() {
            return Ok(Self::from_ansi_index(idx));
        }
        let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if hex.len() != 6 {
            return Err(TuiError::InvalidHexColor {
                input: trimmed.to_owned(),
            });
        }
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| TuiError::InvalidHexColor {
            input: trimmed.to_owned(),
        })?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| TuiError::InvalidHexColor {
            input: trimmed.to_owned(),
        })?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| TuiError::InvalidHexColor {
            input: trimmed.to_owned(),
        })?;
        Ok(Self::Rgb(r, g, b))
    }

    /// Converts an 8-bit palette index into the corresponding semantic 16-color variant or `Indexed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Color;
    ///
    /// assert_eq!(Color::from_ansi_index(10), Color::LightGreen);
    /// assert_eq!(Color::from_ansi_index(200), Color::Indexed(200));
    /// ```
    #[must_use]
    pub const fn from_ansi_index(index: u8) -> Self {
        match index {
            0 => Self::Black,
            1 => Self::Red,
            2 => Self::Green,
            3 => Self::Yellow,
            4 => Self::Blue,
            5 => Self::Magenta,
            6 => Self::Cyan,
            7 => Self::Gray,
            8 => Self::DarkGray,
            9 => Self::LightRed,
            10 => Self::LightGreen,
            11 => Self::LightYellow,
            12 => Self::LightBlue,
            13 => Self::LightMagenta,
            14 => Self::LightCyan,
            15 => Self::White,
            other => Self::Indexed(other),
        }
    }

    /// Degrades this color to match the terminal's maximum [`ColorDepth`].
    ///
    /// Follows `btop_theme.cpp`'s 6×6×6 cube & 24-step grayscale mapping for 256-color
    /// and nearest-perception mapping for 16-color consoles.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, ColorDepth};
    ///
    /// let teal = Color::Rgb(0, 180, 216);
    /// assert!(matches!(teal.degrade(ColorDepth::Ansi256), Color::Indexed(_)));
    /// assert_eq!(teal.degrade(ColorDepth::NoColor), Color::Reset);
    /// ```
    #[must_use]
    pub const fn degrade(self, depth: ColorDepth) -> Self {
        match depth {
            ColorDepth::NoColor => Self::Reset,
            ColorDepth::TrueColor => self,
            ColorDepth::Ansi256 => match self {
                Self::Rgb(r, g, b) => Self::Indexed(rgb_to_ansi256(r, g, b)),
                other => other,
            },
            ColorDepth::Ansi16 => match self {
                Self::Rgb(r, g, b) => rgb_to_ansi16(r, g, b),
                Self::Indexed(idx) => {
                    if idx < 16 {
                        Self::from_ansi_index(idx)
                    } else {
                        let (r, g, b) = ansi256_to_rgb(idx);
                        rgb_to_ansi16(r, g, b)
                    }
                }
                other => other,
            },
        }
    }
}

/// Maps 24-bit `(r, g, b)` to the xterm 256-color cube (`16..=231`) or grayscale ramp (`232..=255`).
#[must_use]
pub const fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        let step = ((r as u16).saturating_sub(8).saturating_mul(24)) / 247;
        #[expect(clippy::cast_possible_truncation, reason = "step is bounded to 0..=23")]
        return 232u8.saturating_add(step as u8);
    }
    let r6 = ((r as u16).saturating_mul(5).saturating_add(127)) / 255;
    let g6 = ((g as u16).saturating_mul(5).saturating_add(127)) / 255;
    let b6 = ((b as u16).saturating_mul(5).saturating_add(127)) / 255;
    let idx = 16u16
        .saturating_add(r6.saturating_mul(36))
        .saturating_add(g6.saturating_mul(6))
        .saturating_add(b6);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "idx is bounded to 16..=231"
    )]
    return idx as u8;
}

/// Approximates an xterm 256-color index (`16..=255`) back into `(r, g, b)`.
#[must_use]
const fn ansi256_to_rgb(idx: u8) -> (u8, u8, u8) {
    if idx >= 232 {
        let gray = (idx.saturating_sub(232))
            .saturating_mul(10)
            .saturating_add(8);
        return (gray, gray, gray);
    }
    let cube = idx.saturating_sub(16);
    let r = (cube / 36) * 51;
    let g = ((cube % 36) / 6) * 51;
    let b = (cube % 6) * 51;
    (r, g, b)
}

/// Maps `(r, g, b)` to one of the 16 standard ANSI colors for Linux virtual consoles.
#[must_use]
pub const fn rgb_to_ansi16(r: u8, g: u8, b: u8) -> Color {
    let max_c = if r >= g && r >= b {
        r
    } else if g >= b {
        g
    } else {
        b
    };
    let min_c = if r <= g && r <= b {
        r
    } else if g <= b {
        g
    } else {
        b
    };

    if max_c < 45 {
        return Color::Black;
    }
    if max_c.saturating_sub(min_c) < 35 {
        if max_c >= 210 {
            return Color::White;
        }
        if max_c >= 135 {
            return Color::Gray;
        }
        return Color::DarkGray;
    }

    let bright = max_c >= 175;
    let r_on = r >= max_c.saturating_sub(55);
    let g_on = g >= max_c.saturating_sub(55);
    let b_on = b >= max_c.saturating_sub(55);

    match (r_on, g_on, b_on, bright) {
        (true, false, false, true) => Color::LightRed,
        (true, false, false, false) => Color::Red,
        (false, true, false, true) => Color::LightGreen,
        (false, true, false, false) => Color::Green,
        (true, true, false, true) => Color::LightYellow,
        (true, true, false, false) => Color::Yellow,
        (false, false, true, true) => Color::LightBlue,
        (false, false, true, false) => Color::Blue,
        (true, false, true, true) => Color::LightMagenta,
        (true, false, true, false) => Color::Magenta,
        (false, true, true, true) => Color::LightCyan,
        (false, true, true, false) => Color::Cyan,
        (_, _, _, true) => Color::White,
        (_, _, _, false) => Color::Gray,
    }
}
