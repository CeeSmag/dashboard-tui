//! Terminal environment and Linux virtual console (`TTY`) capability detection.
//!
//! Detects `NO_COLOR`, `CLOUDVAULT_TTY`, `TERM=linux`, and `/proc/self/fd/{0,1}` symlink targets
//! matching `/dev/tty[0-9]*` (matching `btop`'s `Term::current_tty` and `de_monitors.go`'s `btIsTTY`).

use std::env;
use std::fs;

use crate::core::color::ColorDepth;

/// Hardware and environment capabilities of the active terminal session.
///
/// # Examples
///
/// ```
/// use tui_rs::backend::TerminalCapabilities;
///
/// let caps = TerminalCapabilities::detect();
/// let _ = caps.is_tty_console();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Terminal capability flags are independent boolean feature probes"
)]
#[non_exhaustive]
pub struct TerminalCapabilities {
    color_depth: ColorDepth,
    is_tty_console: bool,
    supports_synchronized_output: bool,
    supports_grapheme_clustering: bool,
    supports_hyperlinks: bool,
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self {
            color_depth: ColorDepth::TrueColor,
            is_tty_console: false,
            supports_synchronized_output: true,
            supports_grapheme_clustering: true,
            supports_hyperlinks: true,
        }
    }
}

impl TerminalCapabilities {
    /// Creates an explicit capability profile (ideal for deterministic headless tests).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    /// use tui_rs::core::ColorDepth;
    ///
    /// let tty_caps = TerminalCapabilities::new(ColorDepth::Ansi16, true, false, false);
    /// assert!(tty_caps.is_tty_console());
    /// ```
    #[must_use]
    pub const fn new(
        color_depth: ColorDepth,
        is_tty_console: bool,
        supports_synchronized_output: bool,
        supports_grapheme_clustering: bool,
    ) -> Self {
        Self {
            color_depth,
            is_tty_console,
            supports_synchronized_output,
            supports_grapheme_clustering,
            supports_hyperlinks: !is_tty_console,
        }
    }

    /// Sets whether OSC 8 hyperlinks are supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    ///
    /// let caps = TerminalCapabilities::default().with_hyperlinks(false);
    /// assert!(!caps.supports_hyperlinks());
    /// ```
    #[must_use]
    pub const fn with_hyperlinks(mut self, supports: bool) -> Self {
        self.supports_hyperlinks = supports;
        self
    }

    /// Probes the current OS environment variables and `/proc/self/fd` to determine capabilities.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    ///
    /// let caps = TerminalCapabilities::detect();
    /// let _ = caps.color_depth();
    /// ```
    #[must_use]
    pub fn detect() -> Self {
        let is_tty = detect_linux_tty_console();
        let color_depth = detect_color_depth(is_tty);
        Self {
            color_depth,
            is_tty_console: is_tty,
            supports_synchronized_output: !is_tty,
            supports_grapheme_clustering: !is_tty,
            supports_hyperlinks: !is_tty,
        }
    }

    /// Returns the detected or configured [`ColorDepth`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    /// use tui_rs::core::ColorDepth;
    ///
    /// assert_eq!(TerminalCapabilities::default().color_depth(), ColorDepth::TrueColor);
    /// ```
    #[must_use]
    pub const fn color_depth(self) -> ColorDepth {
        self.color_depth
    }

    /// Returns true if running on a Linux virtual console (`/dev/tty1..6`, `TERM=linux`)
    /// or `CLOUDVAULT_TTY=1` where Braille glyphs should degrade to CP437 shading blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    /// assert!(!TerminalCapabilities::default().is_tty_console());
    /// ```
    #[must_use]
    pub const fn is_tty_console(self) -> bool {
        self.is_tty_console
    }

    /// Returns true if DEC Private Mode 2026 (`\x1b[?2026h`) synchronized output is supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    /// assert!(TerminalCapabilities::default().supports_synchronized_output());
    /// ```
    #[must_use]
    pub const fn supports_synchronized_output(self) -> bool {
        self.supports_synchronized_output
    }

    /// Returns true if DEC Private Mode 2027 (`\x1b[?2027h`) Unicode grapheme clustering is supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    /// assert!(TerminalCapabilities::default().supports_grapheme_clustering());
    /// ```
    #[must_use]
    pub const fn supports_grapheme_clustering(self) -> bool {
        self.supports_grapheme_clustering
    }

    /// Returns true if OSC 8 clickable terminal hyperlinks (`\x1b]8;;<URL>\x1b\\`) are supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::backend::TerminalCapabilities;
    /// assert!(TerminalCapabilities::default().supports_hyperlinks());
    /// ```
    #[must_use]
    pub const fn supports_hyperlinks(self) -> bool {
        self.supports_hyperlinks
    }
}

/// Detects whether the current process is attached to a Linux virtual console (`btIsTTY` in `de_monitors.go`).
#[must_use]
pub fn detect_linux_tty_console() -> bool {
    if let Ok(val) = env::var("CLOUDVAULT_TTY") {
        let trimmed = val.trim();
        if trimmed == "1" || trimmed.eq_ignore_ascii_case("true") {
            return true;
        }
    }
    if let Ok(val) = env::var("VAULTDRIVE_TTY") {
        let trimmed = val.trim();
        if trimmed == "1" || trimmed.eq_ignore_ascii_case("true") {
            return true;
        }
    }
    if let Ok(term) = env::var("TERM") {
        if term.trim().eq_ignore_ascii_case("linux") {
            return true;
        }
    }
    for fd_path in ["/proc/self/fd/0", "/proc/self/fd/1"] {
        if let Ok(target) = fs::read_link(fd_path) {
            if let Some(path_str) = target.to_str() {
                if path_str.starts_with("/dev/tty") && !path_str.starts_with("/dev/pts/") {
                    return true;
                }
            }
        }
    }
    false
}

/// Determines the appropriate [`ColorDepth`] from `NO_COLOR`, `COLORTERM`, and `TERM`.
#[must_use]
pub fn detect_color_depth(is_tty_console: bool) -> ColorDepth {
    if env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return ColorDepth::NoColor;
    }
    if let Ok(term) = env::var("TERM") {
        if term.trim().eq_ignore_ascii_case("dumb") {
            return ColorDepth::NoColor;
        }
    }
    if is_tty_console {
        return ColorDepth::Ansi16;
    }
    if let Ok(colorterm) = env::var("COLORTERM") {
        let lower = colorterm.to_ascii_lowercase();
        if lower.contains("truecolor") || lower.contains("24bit") {
            return ColorDepth::TrueColor;
        }
    }
    if let Ok(term) = env::var("TERM") {
        if term.contains("256color") {
            return ColorDepth::Ansi256;
        }
    }
    ColorDepth::TrueColor
}
