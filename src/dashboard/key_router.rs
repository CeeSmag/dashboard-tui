//! Keyboard input representation (`KeyPress`, `KeyCode`) and event consumption states.
//!
//! Synthesizes `gitui`'s `EventState` (`Consumed` / `NotConsumed`) with
//! ergonomic conversion from `crossterm::event::KeyEvent`.

use crossterm::event::{KeyCode as CtKeyCode, KeyEvent, KeyModifiers};

use crate::core::StackStr;

/// Indicates whether an input event was handled by a component, modal, or sub-view.
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::EventState;
///
/// assert!(EventState::Consumed.is_consumed());
/// assert!(!EventState::NotConsumed.is_consumed());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventState {
    /// The event was handled and must not propagate further.
    Consumed,
    /// The event was ignored by this receiver and may propagate up the hierarchy.
    NotConsumed,
}

impl EventState {
    /// Returns `true` if the event was consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::EventState;
    ///
    /// assert!(EventState::Consumed.is_consumed());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_consumed(self) -> bool {
        matches!(self, Self::Consumed)
    }
}

/// High-level action returned by [`DashboardEngine::handle_key`](crate::dashboard::DashboardEngine::handle_key).
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::EngineAction;
///
/// assert_eq!(EngineAction::Continue, EngineAction::Continue);
/// ```
///
/// The two exit variants let a host distinguish *leaving the UI* from *terminating the whole
/// application*: a `q`/`Esc` at the dashboard root (and `Ctrl+C`) is [`Self::Back`], the
/// configured quit shortkey (`^Q` by default) is [`Self::Quit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EngineAction {
    /// Continue running the dashboard event loop.
    Continue,
    /// The user asked to go back from the dashboard root (the cancel key with nothing left to
    /// cancel) or pressed `Ctrl+C`. Hosts normally leave the UI but keep their services running.
    Back,
    /// The user pressed the configured quit shortkey. Hosts normally stop their services and exit.
    Quit,
}

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

/// Concise, readable global shortkey bindings for a [`DashboardEngine`](crate::dashboard::DashboardEngine).
///
/// Allows customizing all dashboard navigation and action keys in a few readable chained calls
/// inside `dashboard.rs`.
///
/// Supports entering `ctrl+[key]` bindings (e.g. `"ctrl+q"`, `"^q"`, `"ctrl-x"`) as well as
/// standard keys (`"Tab"`, `"Shift+Tab"`, `"Right"`, `"Left"`, `"Enter"`, `'q'`, etc.).
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::{KeyBindings, KeyCode, KeyPress};
///
/// let keys = KeyBindings::default()
///     .quit("ctrl+q")
///     .page_cycle("Shift+Tab", "Tab")
///     .section_cycle("Right", "Left")
///     .up_down("Up", "Down")
///     .vim_up_down('k', 'j')
///     .activate("Enter")
///     .cancel('q');
/// assert_eq!(keys.quit_press(), KeyPress::char('q').with_ctrl(true));
/// assert_eq!(keys.quit_key(), KeyCode::Char('q'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBindings {
    quit: KeyPress,
    prev_page: KeyPress,
    next_page: KeyPress,
    next_section: KeyPress,
    prev_section: KeyPress,
    cursor_up: KeyPress,
    cursor_down: KeyPress,
    alt_up: Option<KeyPress>,
    alt_down: Option<KeyPress>,
    activate: KeyPress,
    alt_activate: Option<KeyPress>,
    cancel: KeyPress,
    numeric_jump: bool,
    confirm_quit: bool,
}

impl Default for KeyBindings {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl KeyBindings {
    /// Creates the standard default shortkey configuration:
    /// - Quit: `"ctrl+q"` / `^Q`
    /// - Page cycle: `"Shift+Tab"` / `"Tab"` (plus each page's own key hint like `"F1"`..`"F12"`)
    /// - Section cycle: `"Right"` / `"Left"` (`→` / `←`)
    /// - Menu Up/Down: `"Up"` / `"Down"` and `'k'` / `'j'`
    /// - Activate option: `"Enter"`
    /// - Cancel prefix/modal: `'q'` (and `"Esc"`)
    /// - Numeric `1`..`99` direct jump: `true`
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode, KeyPress};
    ///
    /// let keys = KeyBindings::new();
    /// assert_eq!(keys.quit_press(), KeyPress::char('q').with_ctrl(true));
    /// assert_eq!(keys.quit_key(), KeyCode::Char('q'));
    /// ```
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            quit: KeyPress {
                code: KeyCode::Char('q'),
                ctrl: true,
                alt: false,
            },
            prev_page: KeyPress::new(KeyCode::BackTab),
            next_page: KeyPress::new(KeyCode::Tab),
            next_section: KeyPress::new(KeyCode::Right),
            prev_section: KeyPress::new(KeyCode::Left),
            cursor_up: KeyPress::new(KeyCode::Up),
            cursor_down: KeyPress::new(KeyCode::Down),
            alt_up: Some(KeyPress::char('k')),
            alt_down: Some(KeyPress::char('j')),
            activate: KeyPress::new(KeyCode::Enter),
            alt_activate: Some(KeyPress::char(' ')),
            cancel: KeyPress::char('q'),
            numeric_jump: true,
            confirm_quit: false,
        }
    }

    /// Sets the quit shortkey (default: `"ctrl+q"` / `^Q`).
    ///
    /// Developers can configure any `ctrl+[key]` of their liking (e.g. `"ctrl+q"`,
    /// `"^q"`, `"ctrl-x"`, `"ctrl+w"`), or a single character / key code.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode, KeyPress};
    ///
    /// let keys = KeyBindings::default().quit("ctrl+x");
    /// assert_eq!(keys.quit_press(), KeyPress::char('x').with_ctrl(true));
    /// assert_eq!(keys.quit_key(), KeyCode::Char('x'));
    /// ```
    #[must_use]
    pub fn quit(mut self, key: impl IntoKeyPress) -> Self {
        self.quit = key.into_key_press();
        self
    }

    /// Sets the previous-page and next-page cycle shortkeys (default: `"Shift+Tab"` and `"Tab"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().page_cycle('<', '>');
    /// assert_eq!(keys.prev_page_key(), KeyCode::Char('<'));
    /// assert_eq!(keys.next_page_key(), KeyCode::Char('>'));
    /// ```
    #[must_use]
    pub fn page_cycle(mut self, prev: impl IntoKeyPress, next: impl IntoKeyPress) -> Self {
        self.prev_page = prev.into_key_press();
        self.next_page = next.into_key_press();
        self
    }

    /// Sets the forward and backward section focus shortkeys (default: `"Right"` and `"Left"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().section_cycle("Right", "Left");
    /// assert_eq!(keys.next_section_key(), KeyCode::Right);
    /// assert_eq!(keys.prev_section_key(), KeyCode::Left);
    /// ```
    #[must_use]
    pub fn section_cycle(mut self, next: impl IntoKeyPress, prev: impl IntoKeyPress) -> Self {
        self.next_section = next.into_key_press();
        self.prev_section = prev.into_key_press();
        self
    }

    /// Sets the primary Up and Down menu navigation keys (default: `"Up"` and `"Down"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().up_down("Up", "Down");
    /// assert_eq!(keys.cursor_up_key(), KeyCode::Up);
    /// ```
    #[must_use]
    pub fn up_down(mut self, up: impl IntoKeyPress, down: impl IntoKeyPress) -> Self {
        self.cursor_up = up.into_key_press();
        self.cursor_down = down.into_key_press();
        self
    }

    /// Sets the secondary (Vim-style) Up and Down menu navigation keys (default: `'k'` and `'j'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().vim_up_down('w', 's');
    /// assert_eq!(keys.alt_up_key(), Some(KeyCode::Char('w')));
    /// ```
    #[must_use]
    pub fn vim_up_down(mut self, up: impl IntoKeyPress, down: impl IntoKeyPress) -> Self {
        self.alt_up = Some(up.into_key_press());
        self.alt_down = Some(down.into_key_press());
        self
    }

    /// Disables secondary Vim-style Up/Down keys so only the primary `up_down` keys move the cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyBindings;
    ///
    /// let keys = KeyBindings::default().without_vim_keys();
    /// assert!(keys.alt_up_key().is_none());
    /// ```
    #[must_use]
    #[inline]
    pub const fn without_vim_keys(mut self) -> Self {
        self.alt_up = None;
        self.alt_down = None;
        self
    }

    /// Sets the option activation key (default: `"Enter"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().activate("Space");
    /// assert_eq!(keys.activate_key(), KeyCode::Char(' '));
    /// ```
    #[must_use]
    pub fn activate(mut self, key: impl IntoKeyPress) -> Self {
        self.activate = key.into_key_press();
        self
    }

    /// Sets the cancel / back key (default: `'q'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().cancel("Esc");
    /// assert_eq!(keys.cancel_key(), KeyCode::Esc);
    /// ```
    #[must_use]
    pub fn cancel(mut self, key: impl IntoKeyPress) -> Self {
        self.cancel = key.into_key_press();
        self
    }

    /// Sets the secondary activation key (default: `Space`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::{KeyBindings, KeyPress};
    ///
    /// let keys = KeyBindings::default().alt_activate('l');
    /// assert!(keys.matches_activate(KeyPress::char('l')));
    /// assert!(keys.matches_activate(KeyPress::new(dashboard_tui::dashboard::KeyCode::Enter)));
    /// ```
    #[must_use]
    pub fn alt_activate(mut self, key: impl IntoKeyPress) -> Self {
        self.alt_activate = Some(key.into_key_press());
        self
    }

    /// Removes the secondary activation key, leaving only the primary one.
    #[must_use]
    #[inline]
    pub const fn without_alt_activate(mut self) -> Self {
        self.alt_activate = None;
        self
    }

    /// Enables or disables `1`..`99` direct numeric option jumping (default: `true`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyBindings;
    ///
    /// let keys = KeyBindings::default().numeric_jump(false);
    /// assert!(!keys.is_numeric_jump_enabled());
    /// ```
    #[must_use]
    #[inline]
    pub const fn numeric_jump(mut self, enabled: bool) -> Self {
        self.numeric_jump = enabled;
        self
    }

    /// Makes the quit key open a warning popup first: `Enter` confirms the quit,
    /// `q` / `Esc` cancels (default: `false`, quit immediately). `Ctrl+C` always leaves at once ([`EngineAction::Back`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyBindings;
    ///
    /// let keys = KeyBindings::default().quit("ctrl+q").confirm_quit(true);
    /// assert!(keys.is_quit_confirmation_required());
    /// ```
    #[must_use]
    #[inline]
    pub const fn confirm_quit(mut self, required: bool) -> Self {
        self.confirm_quit = required;
        self
    }

    /// Returns `true` if the quit key must be confirmed through a warning popup.
    #[must_use]
    #[inline]
    pub const fn is_quit_confirmation_required(&self) -> bool {
        self.confirm_quit
    }

    /// Returns the configured quit [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn quit_key(&self) -> KeyCode {
        self.quit.code()
    }

    /// Returns the configured quit [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn quit_press(&self) -> KeyPress {
        self.quit
    }

    /// Returns the configured previous-page [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn prev_page_key(&self) -> KeyCode {
        self.prev_page.code()
    }

    /// Returns the configured previous-page [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn prev_page_press(&self) -> KeyPress {
        self.prev_page
    }

    /// Returns the configured next-page [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn next_page_key(&self) -> KeyCode {
        self.next_page.code()
    }

    /// Returns the configured next-page [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn next_page_press(&self) -> KeyPress {
        self.next_page
    }

    /// Returns the configured forward section-cycle [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn next_section_key(&self) -> KeyCode {
        self.next_section.code()
    }

    /// Returns the configured forward section-cycle [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn next_section_press(&self) -> KeyPress {
        self.next_section
    }

    /// Returns the configured backward section-cycle [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn prev_section_key(&self) -> KeyCode {
        self.prev_section.code()
    }

    /// Returns the configured backward section-cycle [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn prev_section_press(&self) -> KeyPress {
        self.prev_section
    }

    /// Returns the primary cursor-up [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn cursor_up_key(&self) -> KeyCode {
        self.cursor_up.code()
    }

    /// Returns the primary cursor-up [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn cursor_up_press(&self) -> KeyPress {
        self.cursor_up
    }

    /// Returns the primary cursor-down [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn cursor_down_key(&self) -> KeyCode {
        self.cursor_down.code()
    }

    /// Returns the primary cursor-down [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn cursor_down_press(&self) -> KeyPress {
        self.cursor_down
    }

    /// Returns the optional secondary cursor-up [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn alt_up_key(&self) -> Option<KeyCode> {
        match self.alt_up {
            Some(kp) => Some(kp.code()),
            None => None,
        }
    }

    /// Returns the optional secondary cursor-up [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn alt_up_press(&self) -> Option<KeyPress> {
        self.alt_up
    }

    /// Returns the optional secondary cursor-down [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn alt_down_key(&self) -> Option<KeyCode> {
        match self.alt_down {
            Some(kp) => Some(kp.code()),
            None => None,
        }
    }

    /// Returns the optional secondary cursor-down [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn alt_down_press(&self) -> Option<KeyPress> {
        self.alt_down
    }

    /// Returns `true` if `key` matches either the primary or secondary cursor-up key.
    #[must_use]
    pub fn is_up(&self, key: impl IntoKeyPress) -> bool {
        let kp = key.into_key_press();
        kp.matches(self.cursor_up) || self.alt_up.is_some_and(|alt| kp.matches(alt))
    }

    /// Returns `true` if `key` matches either the primary or secondary cursor-down key.
    #[must_use]
    pub fn is_down(&self, key: impl IntoKeyPress) -> bool {
        let kp = key.into_key_press();
        kp.matches(self.cursor_down) || self.alt_down.is_some_and(|alt| kp.matches(alt))
    }

    /// Returns the configured option activation [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn activate_key(&self) -> KeyCode {
        self.activate.code()
    }

    /// Returns the configured option activation [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn activate_press(&self) -> KeyPress {
        self.activate
    }

    /// Returns the configured cancel [`KeyCode`].
    #[must_use]
    #[inline]
    pub const fn cancel_key(&self) -> KeyCode {
        self.cancel.code()
    }

    /// Returns the configured cancel [`KeyPress`].
    #[must_use]
    #[inline]
    pub const fn cancel_press(&self) -> KeyPress {
        self.cancel
    }

    /// Returns `true` if `1`..`99` numeric option jumping is enabled.
    #[must_use]
    #[inline]
    pub const fn is_numeric_jump_enabled(&self) -> bool {
        self.numeric_jump
    }

    /// Returns `true` if `key` matches the configured quit key (e.g. `Ctrl+Q` / `^Q`).
    #[must_use]
    pub fn matches_quit(&self, key: KeyPress) -> bool {
        key.matches(self.quit)
    }

    /// Returns `true` if `key` matches the configured cancel key (e.g. `'q'` or `Esc`).
    #[must_use]
    pub fn matches_cancel(&self, key: KeyPress) -> bool {
        key.matches(self.cancel) || key.code() == KeyCode::Esc
    }

    /// Returns `true` if `key` matches the configured next page key (e.g. `Tab`).
    #[must_use]
    pub fn matches_next_page(&self, key: KeyPress) -> bool {
        key.matches(self.next_page)
    }

    /// Returns `true` if `key` matches the configured previous page key (e.g. `Shift+Tab` / `BackTab`).
    #[must_use]
    pub fn matches_prev_page(&self, key: KeyPress) -> bool {
        key.matches(self.prev_page)
    }

    /// Returns `true` if `key` matches the configured next section key (e.g. `Right` / `→`).
    #[must_use]
    pub fn matches_next_section(&self, key: KeyPress) -> bool {
        key.matches(self.next_section)
    }

    /// Returns `true` if `key` matches the configured previous section key (e.g. `Left` / `←`).
    #[must_use]
    pub fn matches_prev_section(&self, key: KeyPress) -> bool {
        key.matches(self.prev_section)
    }

    /// Returns `true` if `key` matches the configured option activation key.
    #[must_use]
    pub fn matches_activate(&self, key: KeyPress) -> bool {
        key.matches(self.activate) || self.alt_activate.is_some_and(|alt| key.matches(alt))
    }
}
