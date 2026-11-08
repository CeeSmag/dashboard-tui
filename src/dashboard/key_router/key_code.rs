//! [`KeyCode`] and its conversion trait.

use crate::core::StackStr;

/// Normalized keyboard key code for dashboard routing.
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::KeyCode;
///
/// let key = KeyCode::Char('1');
/// assert_eq!(key, KeyCode::Char('1'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Printable character key.
    Char(char),
    /// Enter / Return key.
    Enter,
    /// Escape key.
    Esc,
    /// Backspace key.
    Backspace,
    /// Delete key.
    Delete,
    /// Tab key (cycle focus forward).
    Tab,
    /// Shift+Tab / `BackTab` key (cycle focus backward).
    BackTab,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// `PageUp` key.
    PageUp,
    /// `PageDown` key.
    PageDown,
    /// Function key (`F(1)` .. `F(12)`).
    F(u8),
}

impl KeyCode {
    /// Parses a short human-readable key name (e.g. `"q"`, `"Tab"`, `"BackTab"`,
    /// `"Enter"`, `"Esc"`, `"Space"`, `"Up"`, `"Down"`, `"F1"`..`"F12"`) into a [`KeyCode`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyCode;
    ///
    /// assert_eq!(KeyCode::from_hint("F1"), Some(KeyCode::F(1)));
    /// assert_eq!(KeyCode::from_hint("Tab"), Some(KeyCode::Tab));
    /// assert_eq!(KeyCode::from_hint("q"), Some(KeyCode::Char('q')));
    /// ```
    #[must_use]
    pub fn from_hint(hint: &str) -> Option<Self> {
        let s = hint.trim();
        if s.is_empty() {
            return None;
        }
        if s.eq_ignore_ascii_case("enter") || s.eq_ignore_ascii_case("return") {
            return Some(Self::Enter);
        }
        if s.eq_ignore_ascii_case("esc") || s.eq_ignore_ascii_case("escape") {
            return Some(Self::Esc);
        }
        if s.eq_ignore_ascii_case("tab") {
            return Some(Self::Tab);
        }
        if s.eq_ignore_ascii_case("backtab") || s.eq_ignore_ascii_case("shift+tab") {
            return Some(Self::BackTab);
        }
        if s.eq_ignore_ascii_case("space") {
            return Some(Self::Char(' '));
        }
        if s.eq_ignore_ascii_case("backspace") {
            return Some(Self::Backspace);
        }
        if s.eq_ignore_ascii_case("delete") || s.eq_ignore_ascii_case("del") {
            return Some(Self::Delete);
        }
        if s.eq_ignore_ascii_case("up") || s == "↑" {
            return Some(Self::Up);
        }
        if s.eq_ignore_ascii_case("down") || s == "↓" {
            return Some(Self::Down);
        }
        if s.eq_ignore_ascii_case("left") || s == "←" {
            return Some(Self::Left);
        }
        if s.eq_ignore_ascii_case("right") || s == "→" {
            return Some(Self::Right);
        }
        if s.eq_ignore_ascii_case("home") {
            return Some(Self::Home);
        }
        if s.eq_ignore_ascii_case("end") {
            return Some(Self::End);
        }
        if s.eq_ignore_ascii_case("pageup") || s.eq_ignore_ascii_case("pgup") {
            return Some(Self::PageUp);
        }
        if s.eq_ignore_ascii_case("pagedown") || s.eq_ignore_ascii_case("pgdn") {
            return Some(Self::PageDown);
        }
        if let Some(rest) = s.strip_prefix('F').or_else(|| s.strip_prefix('f')) {
            if let Ok(num) = rest.parse::<u8>() {
                if (1..=12).contains(&num) {
                    return Some(Self::F(num));
                }
            }
        }
        let mut chars = s.chars();
        if let (Some(ch), None) = (chars.next(), chars.next()) {
            return Some(Self::Char(ch));
        }
        None
    }

    /// Returns a compact, stack-allocated label for displaying this key in the footer bar.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyCode;
    ///
    /// assert_eq!(KeyCode::Tab.short_label().as_str(), "Tab");
    /// assert_eq!(KeyCode::Char('q').short_label().as_str(), "q");
    /// ```
    #[must_use]
    pub fn short_label(self) -> StackStr<16> {
        let mut out = StackStr::<16>::new();
        match self {
            Self::Char(' ') => out.push_str("Space"),
            Self::Char(c) => {
                let _ = out.push_char(c);
            }
            Self::Enter => out.push_str("Enter"),
            Self::Esc => out.push_str("Esc"),
            Self::Backspace => out.push_str("Bksp"),
            Self::Delete => out.push_str("Del"),
            Self::Tab => out.push_str("Tab"),
            Self::BackTab => out.push_str("S-Tab"),
            Self::Up => out.push_str("↑"),
            Self::Down => out.push_str("↓"),
            Self::Left => out.push_str("←"),
            Self::Right => out.push_str("→"),
            Self::Home => out.push_str("Home"),
            Self::End => out.push_str("End"),
            Self::PageUp => out.push_str("PgUp"),
            Self::PageDown => out.push_str("PgDn"),
            Self::F(n) => {
                let _ = out.push_char('F');
                if n >= 10 {
                    let _ = out.push_char(char::from(b'0' + (n / 10)));
                }
                let _ = out.push_char(char::from(b'0' + (n % 10)));
            }
        }
        out
    }
}

/// Conversion trait allowing shortkeys to be specified concisely as `'q'`, `"Tab"`, or [`KeyCode::Tab`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::{IntoKeyCode, KeyCode};
///
/// assert_eq!('q'.into_key_code(), KeyCode::Char('q'));
/// assert_eq!("Enter".into_key_code(), KeyCode::Enter);
/// ```
pub trait IntoKeyCode {
    /// Converts `self` into a [`KeyCode`].
    #[must_use]
    fn into_key_code(self) -> KeyCode;
}

impl IntoKeyCode for KeyCode {
    #[inline]
    fn into_key_code(self) -> KeyCode {
        self
    }
}

impl IntoKeyCode for char {
    #[inline]
    fn into_key_code(self) -> KeyCode {
        KeyCode::Char(self)
    }
}

impl IntoKeyCode for &str {
    #[inline]
    fn into_key_code(self) -> KeyCode {
        KeyCode::from_hint(self).unwrap_or(KeyCode::Esc)
    }
}
