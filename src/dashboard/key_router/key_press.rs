//! [`KeyPress`] (code plus modifiers) and its conversion trait.

use crossterm::event::{KeyCode as CtKeyCode, KeyEvent, KeyModifiers};

use super::key_code::KeyCode;
use crate::core::StackStr;

/// Normalized keyboard press event combining [`KeyCode`] with control/alt flags.
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::{KeyCode, KeyPress};
///
/// let kp = KeyPress::char('q');
/// assert_eq!(kp.code(), KeyCode::Char('q'));
/// assert!(!kp.has_ctrl());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyPress {
    code: KeyCode,
    ctrl: bool,
    alt: bool,
}

impl KeyPress {
    /// Creates a new [`KeyPress`] with no modifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyCode, KeyPress};
    ///
    /// let kp = KeyPress::new(KeyCode::Enter);
    /// assert_eq!(kp.code(), KeyCode::Enter);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: false,
        }
    }

    /// Creates a character [`KeyPress`] without modifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyCode, KeyPress};
    ///
    /// let kp = KeyPress::char('5');
    /// assert_eq!(kp.code(), KeyCode::Char('5'));
    /// ```
    #[must_use]
    #[inline]
    pub const fn char(ch: char) -> Self {
        Self::new(KeyCode::Char(ch))
    }

    /// Marks the key press as having the `Ctrl` modifier held.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyPress;
    ///
    /// let kp = KeyPress::char('c').with_ctrl(true);
    /// assert!(kp.has_ctrl());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_ctrl(mut self, ctrl: bool) -> Self {
        self.ctrl = ctrl;
        self
    }

    /// Marks the key press as having the `Alt` modifier held.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyPress;
    ///
    /// let kp = KeyPress::char('x').with_alt(true);
    /// assert!(kp.has_alt());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_alt(mut self, alt: bool) -> Self {
        self.alt = alt;
        self
    }

    /// Returns the [`KeyCode`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyCode, KeyPress};
    ///
    /// assert_eq!(KeyPress::new(KeyCode::Esc).code(), KeyCode::Esc);
    /// ```
    #[must_use]
    #[inline]
    pub const fn code(&self) -> KeyCode {
        self.code
    }

    /// Returns `true` if the `Ctrl` modifier is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyPress;
    ///
    /// assert!(!KeyPress::char('a').has_ctrl());
    /// ```
    #[must_use]
    #[inline]
    pub const fn has_ctrl(&self) -> bool {
        self.ctrl
    }

    /// Returns `true` if the `Alt` modifier is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyPress;
    ///
    /// assert!(!KeyPress::char('a').has_alt());
    /// ```
    #[must_use]
    #[inline]
    pub const fn has_alt(&self) -> bool {
        self.alt
    }

    /// Returns `true` if this [`KeyPress`] matches `other`, comparing characters case-insensitively.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyPress;
    ///
    /// let kp1 = KeyPress::char('q').with_ctrl(true);
    /// let kp2 = KeyPress::char('Q').with_ctrl(true);
    /// assert!(kp1.matches(kp2));
    /// ```
    #[must_use]
    pub fn matches(&self, other: Self) -> bool {
        self.ctrl == other.ctrl
            && self.alt == other.alt
            && match (self.code, other.code) {
                (KeyCode::Char(a), KeyCode::Char(b)) => a.eq_ignore_ascii_case(&b),
                (a, b) => a == b,
            }
    }

    /// Returns a compact, stack-allocated label for displaying this key in the footer bar.
    ///
    /// Formats control keys in standard terminal caret notation (`"^Q"`, `^C`) or `"C-Tab"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyCode, KeyPress};
    ///
    /// assert_eq!(KeyPress::char('q').with_ctrl(true).short_label().as_str(), "^Q");
    /// assert_eq!(KeyPress::new(KeyCode::Right).short_label().as_str(), "→");
    /// ```
    #[must_use]
    pub fn short_label(&self) -> StackStr<16> {
        let mut out = StackStr::<16>::new();
        if self.ctrl {
            if let KeyCode::Char(c) = self.code {
                let _ = out.push_char('^');
                let _ = out.push_char(c.to_ascii_uppercase());
                return out;
            }
            out.push_str("C-");
        }
        if self.alt {
            out.push_str("A-");
        }
        out.push_str(self.code.short_label().as_str());
        out
    }

    /// Parses a human-readable key specification, supporting `ctrl+[key]` (`"ctrl+q"`,
    /// `"^q"`, `"Ctrl-Q"`), `alt+[key]`, `shift+[key]`, and plain keys (`"Tab"`, `"Enter"`,
    /// `"Right"`, `"Left"`, `"q"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyCode, KeyPress};
    ///
    /// assert_eq!(KeyPress::from_hint("ctrl+q"), Some(KeyPress::char('q').with_ctrl(true)));
    /// assert_eq!(KeyPress::from_hint("^q"), Some(KeyPress::char('q').with_ctrl(true)));
    /// assert_eq!(KeyPress::from_hint("Shift+Tab"), Some(KeyPress::new(KeyCode::BackTab)));
    /// assert_eq!(KeyPress::from_hint("Right"), Some(KeyPress::new(KeyCode::Right)));
    /// ```
    #[must_use]
    pub fn from_hint(hint: &str) -> Option<Self> {
        let s = hint.trim();
        if s.is_empty() {
            return None;
        }

        // Caret notation e.g. "^q", "^Q", "^c"
        if let Some(rest) = s.strip_prefix('^') {
            let inner = rest.trim();
            if let Some(code) = KeyCode::from_hint(inner) {
                let normalized_code = match code {
                    KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                        KeyCode::Char(c.to_ascii_lowercase())
                    }
                    other => other,
                };
                return Some(Self::new(normalized_code).with_ctrl(true));
            }
            let mut chars = inner.chars();
            if let (Some(ch), None) = (chars.next(), chars.next()) {
                return Some(Self::char(ch.to_ascii_lowercase()).with_ctrl(true));
            }
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut rem = s;

        loop {
            let lower = rem.to_ascii_lowercase();
            if let Some(rest) = lower
                .strip_prefix("ctrl+")
                .or_else(|| lower.strip_prefix("ctrl-"))
                .or_else(|| lower.strip_prefix("control+"))
                .or_else(|| lower.strip_prefix("control-"))
            {
                ctrl = true;
                rem = &rem[rem.len() - rest.len()..];
            } else if let Some(rest) = lower
                .strip_prefix("alt+")
                .or_else(|| lower.strip_prefix("alt-"))
            {
                alt = true;
                rem = &rem[rem.len() - rest.len()..];
            } else if let Some(rest) = lower
                .strip_prefix("shift+")
                .or_else(|| lower.strip_prefix("shift-"))
            {
                if rest.eq_ignore_ascii_case("tab") {
                    return Some(Self::new(KeyCode::BackTab).with_ctrl(ctrl).with_alt(alt));
                }
                rem = &rem[rem.len() - rest.len()..];
            } else {
                break;
            }
        }

        let rem = rem.trim();
        if let Some(code) = KeyCode::from_hint(rem) {
            let normalized_code = match code {
                KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                    KeyCode::Char(c.to_ascii_lowercase())
                }
                other => other,
            };
            return Some(Self::new(normalized_code).with_ctrl(ctrl).with_alt(alt));
        }
        let mut chars = rem.chars();
        if let (Some(ch), None) = (chars.next(), chars.next()) {
            return Some(
                Self::char(ch.to_ascii_lowercase())
                    .with_ctrl(ctrl)
                    .with_alt(alt),
            );
        }
        None
    }

    /// Converts a `crossterm::event::KeyEvent` into a [`KeyPress`], returning `None`
    /// for unsupported media/modifier-only keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::{KeyCode as CtKeyCode, KeyEvent, KeyModifiers};
    /// use dashboard_tui::dashboard::{KeyCode, KeyPress};
    ///
    /// let ev = KeyEvent::new(CtKeyCode::Char('q'), KeyModifiers::NONE);
    /// let kp = KeyPress::from_crossterm(ev);
    /// assert_eq!(kp, Some(KeyPress::char('q')));
    /// ```
    #[must_use]
    pub const fn from_crossterm(event: KeyEvent) -> Option<Self> {
        let code = match event.code {
            CtKeyCode::Char(c) => KeyCode::Char(c),
            CtKeyCode::Enter => KeyCode::Enter,
            CtKeyCode::Esc => KeyCode::Esc,
            CtKeyCode::Backspace => KeyCode::Backspace,
            CtKeyCode::Delete => KeyCode::Delete,
            CtKeyCode::Tab => {
                if event.modifiers.contains(KeyModifiers::SHIFT) {
                    KeyCode::BackTab
                } else {
                    KeyCode::Tab
                }
            }
            CtKeyCode::BackTab => KeyCode::BackTab,
            CtKeyCode::Up => KeyCode::Up,
            CtKeyCode::Down => KeyCode::Down,
            CtKeyCode::Left => KeyCode::Left,
            CtKeyCode::Right => KeyCode::Right,
            CtKeyCode::Home => KeyCode::Home,
            CtKeyCode::End => KeyCode::End,
            CtKeyCode::PageUp => KeyCode::PageUp,
            CtKeyCode::PageDown => KeyCode::PageDown,
            CtKeyCode::F(n) => KeyCode::F(n),
            _ => return None,
        };
        Some(Self {
            code,
            ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
            alt: event.modifiers.contains(KeyModifiers::ALT),
        })
    }
}

/// Conversion trait allowing shortkeys to be specified concisely as `'q'`, `"Tab"`,
/// `"ctrl+q"`, `"^q"`, [`KeyCode::Tab`], or [`KeyPress`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::{IntoKeyPress, KeyCode, KeyPress};
///
/// assert_eq!('q'.into_key_press(), KeyPress::char('q'));
/// assert_eq!("ctrl+q".into_key_press(), KeyPress::char('q').with_ctrl(true));
/// assert_eq!("^q".into_key_press(), KeyPress::char('q').with_ctrl(true));
/// assert_eq!("Enter".into_key_press(), KeyPress::new(KeyCode::Enter));
/// ```
pub trait IntoKeyPress {
    /// Converts `self` into a [`KeyPress`].
    #[must_use]
    fn into_key_press(self) -> KeyPress;
}

impl IntoKeyPress for KeyPress {
    #[inline]
    fn into_key_press(self) -> KeyPress {
        self
    }
}

impl IntoKeyPress for KeyCode {
    #[inline]
    fn into_key_press(self) -> KeyPress {
        KeyPress::new(self)
    }
}

impl IntoKeyPress for char {
    #[inline]
    fn into_key_press(self) -> KeyPress {
        KeyPress::char(self)
    }
}

impl IntoKeyPress for &str {
    #[inline]
    fn into_key_press(self) -> KeyPress {
        KeyPress::from_hint(self).unwrap_or(KeyPress::new(KeyCode::Esc))
    }
}

impl IntoKeyPress for &String {
    #[inline]
    fn into_key_press(self) -> KeyPress {
        self.as_str().into_key_press()
    }
}
