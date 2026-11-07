//! Semantic color palettes and `btop`-style 101-entry precomputed thermal gradient lookup tables.
//!
//! Includes built-in palettes matching `ref/dashboard` and `ref/duf-master`:
//! - **`CloudVault` Teal** (`default`)
//! - **Catppuccin Mocha** (`catppuccin`)
//! - **Dracula** (`dracula`)
//! - **Nord** (`nord`)
//! - **Light** (`light`)
//! - **Linux Console TTY 16-Color** (`tty`)

use crate::core::color::Color;
use crate::core::style::Style;

/// Precomputed 101-entry (`0..=100%`) color lookup table for $O(1)$ hot-path meter and waveform styling.
///
/// Inspired by `btop_theme.hpp` (`array<string, 101> gradients`) and `de_theme.go` (`btGradientRGB`).
///
/// # Examples
///
/// ```
/// use tui_rs::core::ColorPalette;
///
/// let lut = ColorPalette::DEFAULT.gradient_lut();
/// let low = lut.color_at_percent(0);
/// let peak = lut.color_at_percent(100);
/// assert_ne!(low, peak);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradientLut {
    table: [Color; 101],
}

impl GradientLut {
    /// Builds a 101-entry gradient LUT across the 4 RGB stops (`low -> mid -> warn -> peak`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Color, GradientLut};
    ///
    /// let lut = GradientLut::from_stops([0, 0, 0], [50, 50, 50], [150, 150, 150], [255, 255, 255]);
    /// assert_eq!(lut.color_at_percent(0), Color::Rgb(0, 0, 0));
    /// assert_eq!(lut.color_at_percent(100), Color::Rgb(255, 255, 255));
    /// ```
    #[must_use]
    pub const fn from_stops(low: [u8; 3], mid: [u8; 3], warn: [u8; 3], peak: [u8; 3]) -> Self {
        let mut table = [Color::Reset; 101];
        let mut pct = 0usize;
        while pct <= 100 {
            #[expect(clippy::cast_possible_truncation, reason = "pct is bounded to 0..=100")]
            let p = pct as u16;
            let rgb = if p == 0 {
                low
            } else if p >= 100 {
                peak
            } else if p < 50 {
                lerp_rgb_u16(low, mid, p, 50)
            } else if p < 80 {
                lerp_rgb_u16(mid, warn, p.saturating_sub(50), 30)
            } else {
                lerp_rgb_u16(warn, peak, p.saturating_sub(80), 20)
            };
            table[pct] = Color::Rgb(rgb[0], rgb[1], rgb[2]);
            pct += 1;
        }
        Self { table }
    }

    /// Returns the precomputed [`Color`] for an integer percentage `0..=100` (clamped).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::ColorPalette;
    ///
    /// let lut = ColorPalette::DEFAULT.gradient_lut();
    /// assert_eq!(lut.color_at_percent(150), lut.color_at_percent(100));
    /// ```
    #[must_use]
    pub const fn color_at_percent(&self, percent: u8) -> Color {
        let idx = if percent > 100 { 100 } else { percent as usize };
        self.table[idx]
    }

    /// Returns the precomputed [`Color`] for a normalized `f32` ratio `0.0..=1.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::ColorPalette;
    ///
    /// let lut = ColorPalette::DEFAULT.gradient_lut();
    /// assert_eq!(lut.at_ratio(0.0), lut.color_at_percent(0));
    /// assert_eq!(lut.at_ratio(1.0), lut.color_at_percent(100));
    /// ```
    #[must_use]
    pub fn at_ratio(&self, ratio: f32) -> Color {
        let clamped = if ratio.is_nan() || ratio <= 0.0 {
            0.0
        } else if ratio >= 1.0 {
            1.0
        } else {
            ratio
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped * 100.0 + 0.5 is guaranteed to be within 0.0..=100.5"
        )]
        let pct = (clamped * 100.0 + 0.5) as u8;
        self.color_at_percent(pct)
    }

    /// Returns the precomputed [`Style`] (foreground color) for a normalized ratio `0.0..=1.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::ColorPalette;
    ///
    /// let lut = ColorPalette::DEFAULT.gradient_lut();
    /// let style = lut.style_at_ratio(0.5);
    /// assert!(style.foreground().is_some());
    /// ```
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "ratio clamped to 0.0..=1.0"
    )]
    pub fn style_at_ratio(&self, ratio: f64) -> Style {
        Style::new().fg(self.at_ratio(ratio as f32))
    }
}

/// Pure integer linear interpolation between two RGB triples (`num / den`), avoiding float math in `const fn`.
#[must_use]
const fn lerp_rgb_u16(a: [u8; 3], b: [u8; 3], num: u16, den: u16) -> [u8; 3] {
    [
        lerp_channel(a[0], b[0], num, den),
        lerp_channel(a[1], b[1], num, den),
        lerp_channel(a[2], b[2], num, den),
    ]
}

#[must_use]
const fn lerp_channel(start: u8, end: u8, num: u16, den: u16) -> u8 {
    if den == 0 {
        return start;
    }
    if end >= start {
        let diff = (end - start) as u16;
        let delta = (diff.saturating_mul(num).saturating_add(den / 2)) / den;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "start + delta <= end <= 255"
        )]
        return start.saturating_add(delta as u8);
    }
    let diff = (start - end) as u16;
    let delta = (diff.saturating_mul(num).saturating_add(den / 2)) / den;
    #[expect(clippy::cast_possible_truncation, reason = "start - delta >= end >= 0")]
    start.saturating_sub(delta as u8)
}

/// Complete semantic color palette for the TUI and Dashboard Engine.
///
/// # Examples
///
/// ```
/// use tui_rs::core::ColorPalette;
///
/// let theme = ColorPalette::lookup("catppuccin");
/// assert_eq!(theme.id(), "catppuccin");
/// assert_eq!(theme.name(), "Catppuccin Mocha");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ColorPalette {
    /// Machine-readable theme identifier.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Short aesthetic description.
    pub vibe: &'static str,
    /// Unfocused panel border color.
    pub border: Color,
    /// Focused panel border color.
    pub border_focus: Color,
    /// Primary accent color (cyan/blue).
    pub accent: Color,
    /// Secondary bright accent color (mint/white-cyan for active selections).
    pub accent2: Color,
    /// Informational accent color.
    pub info: Color,
    /// Success / online green color.
    pub ok: Color,
    /// Warning amber/orange color.
    pub warn: Color,
    /// Error / danger red color.
    pub err: Color,
    /// Primary foreground text color.
    pub text: Color,
    /// Secondary muted label color.
    pub muted: Color,
    /// Dimmed track / inactive backdrop color.
    pub dim: Color,
    /// Selected row background highlight color.
    pub highlight_bg: Color,
    lut: GradientLut,
}

impl Default for ColorPalette {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ColorPalette {
    /// Default **`CloudVault` Teal** palette (`#00B4D8` border with Tailwind accents).
    pub const DEFAULT: Self = Self::from_const(
        "default",
        "CloudVault Teal",
        "Classic electric cyan frame with Tailwind accents",
        Color::Rgb(0x2A, 0x4A, 0x5E),
        Color::Rgb(0x00, 0xB4, 0xD8),
        Color::Rgb(0x38, 0xBD, 0xF8),
        Color::Rgb(0x4A, 0xDE, 0x80),
        Color::Rgb(0x00, 0xB4, 0xD8),
        Color::Rgb(0x22, 0xC5, 0x5E),
        Color::Rgb(0xFB, 0xBF, 0x24),
        Color::Rgb(0xEF, 0x44, 0x44),
        Color::Rgb(0xF8, 0xFA, 0xFC),
        Color::Rgb(0x94, 0xA3, 0xB8),
        Color::Rgb(0x47, 0x55, 0x69),
        Color::Rgb(0x1E, 0x29, 0x3B),
        [56, 189, 248],
        [34, 197, 94],
        [245, 158, 11],
        [239, 68, 68],
    );

    /// **Catppuccin Mocha** palette.
    pub const CATPPUCCIN: Self = Self::from_const(
        "catppuccin",
        "Catppuccin Mocha",
        "Soothing pastel dark aesthetic with soft accents",
        Color::Rgb(0x45, 0x47, 0x5A),
        Color::Rgb(0x89, 0xB4, 0xFA),
        Color::Rgb(0x89, 0xDC, 0xEB),
        Color::Rgb(0x94, 0xE2, 0xD5),
        Color::Rgb(0x74, 0xC7, 0xEC),
        Color::Rgb(0xA6, 0xE3, 0xA1),
        Color::Rgb(0xF9, 0xE2, 0xAF),
        Color::Rgb(0xF3, 0x8B, 0xA8),
        Color::Rgb(0xCD, 0xD6, 0xF4),
        Color::Rgb(0x7F, 0x84, 0x9C),
        Color::Rgb(0x58, 0x5B, 0x70),
        Color::Rgb(0x31, 0x32, 0x44),
        [137, 220, 235],
        [166, 227, 161],
        [249, 226, 175],
        [243, 139, 168],
    );

    /// **Dracula** high-contrast dark palette.
    pub const DRACULA: Self = Self::from_const(
        "dracula",
        "Dracula",
        "Vibrant high-contrast gothic dark theme",
        Color::Rgb(0x62, 0x72, 0xA4),
        Color::Rgb(0xBD, 0x93, 0xF9),
        Color::Rgb(0x8B, 0xE9, 0xFD),
        Color::Rgb(0xFF, 0x79, 0xC6),
        Color::Rgb(0x8B, 0xE9, 0xFD),
        Color::Rgb(0x50, 0xFA, 0x7B),
        Color::Rgb(0xF1, 0xFA, 0x8C),
        Color::Rgb(0xFF, 0x55, 0x55),
        Color::Rgb(0xF8, 0xF8, 0xF2),
        Color::Rgb(0x62, 0x72, 0xA4),
        Color::Rgb(0x44, 0x47, 0x5A),
        Color::Rgb(0x44, 0x47, 0x5A),
        [139, 233, 253],
        [80, 250, 123],
        [241, 250, 140],
        [255, 85, 85],
    );

    /// **Nord** arctic blue minimalist palette.
    pub const NORD: Self = Self::from_const(
        "nord",
        "Nord",
        "Arctic minimalist palette of cool frost blues",
        Color::Rgb(0x4C, 0x56, 0x6A),
        Color::Rgb(0x81, 0xA1, 0xC1),
        Color::Rgb(0x88, 0xC0, 0xD0),
        Color::Rgb(0x8F, 0xBC, 0xBB),
        Color::Rgb(0x5E, 0x81, 0xAC),
        Color::Rgb(0xA3, 0xBE, 0x8C),
        Color::Rgb(0xEB, 0xCB, 0x8B),
        Color::Rgb(0xBF, 0x61, 0x6A),
        Color::Rgb(0xEC, 0xEF, 0xF4),
        Color::Rgb(0x61, 0x6E, 0x88),
        Color::Rgb(0x43, 0x4C, 0x5E),
        Color::Rgb(0x3B, 0x42, 0x52),
        [136, 192, 208],
        [163, 190, 140],
        [235, 203, 139],
        [191, 97, 106],
    );

    /// **Light** daylight high-contrast palette (from `ref/duf-master`).
    pub const LIGHT: Self = Self::from_const(
        "light",
        "Daylight High-Contrast",
        "Crisp daytime palette for light terminal backgrounds",
        Color::Rgb(0x8A, 0x8A, 0x8A),
        Color::Rgb(0x00, 0x87, 0xFF),
        Color::Rgb(0x00, 0x5F, 0xAF),
        Color::Rgb(0x00, 0x87, 0x5F),
        Color::Rgb(0x00, 0x5F, 0xAF),
        Color::Rgb(0x00, 0x5F, 0x00),
        Color::Rgb(0xD7, 0x87, 0x00),
        Color::Rgb(0xD7, 0x00, 0x00),
        Color::Rgb(0x1C, 0x1C, 0x1C),
        Color::Rgb(0x58, 0x58, 0x58),
        Color::Rgb(0x9E, 0x9E, 0x9E),
        Color::Rgb(0xD0, 0xD0, 0xD0),
        [0, 135, 255],
        [0, 135, 95],
        [215, 135, 0],
        [215, 0, 0],
    );

    /// **Linux Console (16 Colors)** native palette for VGA TTY and serial terminals.
    pub const TTY: Self = Self::from_const(
        "tty",
        "Linux Console (16 Colors)",
        "Native 16-color ANSI palette for VGA console TTY and serial terminals",
        Color::DarkGray,
        Color::Cyan,
        Color::LightCyan,
        Color::LightGreen,
        Color::Cyan,
        Color::Green,
        Color::LightYellow,
        Color::LightRed,
        Color::White,
        Color::Gray,
        Color::DarkGray,
        Color::Blue,
        [0, 255, 255],
        [0, 255, 0],
        [255, 255, 0],
        [255, 0, 0],
    );

    /// Array of all 6 built-in themes in display order.
    pub const ALL_THEMES: [Self; 6] = [
        Self::DEFAULT,
        Self::CATPPUCCIN,
        Self::DRACULA,
        Self::NORD,
        Self::LIGHT,
        Self::TTY,
    ];

    #[expect(
        clippy::too_many_arguments,
        reason = "internal const constructor for static palettes"
    )]
    #[must_use]
    const fn from_const(
        id: &'static str,
        name: &'static str,
        vibe: &'static str,
        border: Color,
        border_focus: Color,
        accent: Color,
        accent2: Color,
        info: Color,
        ok: Color,
        warn: Color,
        err: Color,
        text: Color,
        muted: Color,
        dim: Color,
        highlight_bg: Color,
        grad_low: [u8; 3],
        grad_mid: [u8; 3],
        grad_warn: [u8; 3],
        grad_peak: [u8; 3],
    ) -> Self {
        let lut = GradientLut::from_stops(grad_low, grad_mid, grad_warn, grad_peak);
        Self {
            id,
            name,
            vibe,
            border,
            border_focus,
            accent,
            accent2,
            info,
            ok,
            warn,
            err,
            text,
            muted,
            dim,
            highlight_bg,
            lut,
        }
    }

    /// Returns the built-in palettes in standard display order.
    #[must_use]
    pub const fn built_in_palettes() -> [Self; 6] {
        Self::ALL_THEMES
    }

    /// Looks up a built-in palette by case-insensitive ID, falling back to [`Self::DEFAULT`].
    #[must_use]
    pub fn lookup(id: impl AsRef<str>) -> Self {
        let normalized = id.as_ref().trim();
        if normalized.eq_ignore_ascii_case("catppuccin") {
            Self::CATPPUCCIN
        } else if normalized.eq_ignore_ascii_case("dracula") {
            Self::DRACULA
        } else if normalized.eq_ignore_ascii_case("nord") {
            Self::NORD
        } else if normalized.eq_ignore_ascii_case("light") {
            Self::LIGHT
        } else if normalized.eq_ignore_ascii_case("tty") {
            Self::TTY
        } else {
            Self::DEFAULT
        }
    }

    /// Returns the machine-readable palette identifier (e.g., `"catppuccin"`).
    #[must_use]
    #[inline]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Returns the human-readable display name (e.g., `"CloudVault Teal"`).
    #[must_use]
    #[inline]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the descriptive vibe subtitle of this palette.
    #[must_use]
    #[inline]
    pub const fn vibe(&self) -> &'static str {
        self.vibe
    }

    /// Returns the precomputed 101-entry [`GradientLut`].
    #[must_use]
    #[inline]
    pub const fn gradient_lut(&self) -> &GradientLut {
        &self.lut
    }

    /// Panel border style (`stBorder`).
    #[must_use]
    pub const fn border_style(&self) -> Style {
        Style::new().fg(self.border)
    }

    /// Bold panel title style (`stTitle`).
    #[must_use]
    pub const fn title_style(&self) -> Style {
        Style::new().fg(self.accent).bold()
    }

    /// Regular cyan accent style (`stCyan`).
    #[must_use]
    pub const fn cyan_style(&self) -> Style {
        Style::new().fg(self.accent)
    }

    /// Bold key shortcut & pointer style (`stKey`).
    #[must_use]
    pub const fn key_style(&self) -> Style {
        Style::new().fg(self.accent).bold()
    }

    /// Bold green online/success style (`stGreen`).
    #[must_use]
    pub const fn green_style(&self) -> Style {
        Style::new().fg(self.ok).bold()
    }

    /// Secondary mint accent style (`stMint`).
    #[must_use]
    pub const fn mint_style(&self) -> Style {
        Style::new().fg(self.accent2)
    }

    /// Warning amber style (`stAmber`).
    #[must_use]
    pub const fn amber_style(&self) -> Style {
        Style::new().fg(self.warn)
    }

    /// Orange alert style (`stOrange`).
    #[must_use]
    pub const fn orange_style(&self) -> Style {
        Style::new().fg(self.warn)
    }

    /// Bold red error/danger style (`stRed`).
    #[must_use]
    pub const fn red_style(&self) -> Style {
        Style::new().fg(self.err).bold()
    }

    /// Muted slate/gray style (`stMuted`).
    #[must_use]
    pub const fn muted_style(&self) -> Style {
        Style::new().fg(self.muted)
    }

    /// Bold bright white primary text/cursor style (`stWhite`).
    #[must_use]
    pub const fn white_style(&self) -> Style {
        Style::new().fg(self.text).bold()
    }

    /// Unfilled meter/slider track style (`stTrack`).
    #[must_use]
    pub const fn track_style(&self) -> Style {
        Style::new().fg(self.dim)
    }

    /// Returns the 6 swatch colors for palette previews.
    #[must_use]
    pub const fn swatch_colors(&self) -> [Color; 6] {
        [
            self.border_focus,
            self.accent,
            self.ok,
            self.warn,
            self.err,
            self.text,
        ]
    }
}
