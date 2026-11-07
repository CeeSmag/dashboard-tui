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
/// use tui_rs::dashboard::EventState;
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
    /// use tui_rs::dashboard::EventState;
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
/// use tui_rs::dashboard::EngineAction;
///
/// assert_eq!(EngineAction::Continue, EngineAction::Continue);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineAction {
    /// Continue running the dashboard event loop.
    Continue,
    /// The user requested to exit the dashboard (e.g., `q` or `Ctrl+C`).
    Quit,
}

/// Normalized keyboard key code for dashboard routing.
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::KeyCode;
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
    /// use tui_rs::dashboard::KeyCode;
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
    /// use tui_rs::dashboard::KeyCode;
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
/// use tui_rs::dashboard::{IntoKeyCode, KeyCode};
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

/// Concise, readable global shortkey bindings for a [`DashboardEngine`](crate::dashboard::DashboardEngine).
///
/// Allows customizing all dashboard navigation and action keys in a few readable chained calls
/// inside `dashboard.rs`.
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::{KeyBindings, KeyCode};
///
/// let keys = KeyBindings::default()
///     .quit('x')
///     .page_cycle('<', '>')
///     .section_cycle("Tab", "BackTab")
///     .up_down("Up", "Down")
///     .vim_up_down('k', 'j')
///     .activate("Enter")
///     .cancel("Esc");
/// assert_eq!(keys.quit_key(), KeyCode::Char('x'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBindings {
    quit: KeyCode,
    prev_page: KeyCode,
    next_page: KeyCode,
    next_section: KeyCode,
    prev_section: KeyCode,
    cursor_up: KeyCode,
    cursor_down: KeyCode,
    alt_up: Option<KeyCode>,
    alt_down: Option<KeyCode>,
    activate: KeyCode,
    cancel: KeyCode,
    numeric_jump: bool,
}

impl Default for KeyBindings {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl KeyBindings {
    /// Creates the standard default shortkey configuration:
    /// - Quit: `'q'`
    /// - Page cycle: `'['` / `']'` (plus each page's own key hint like `"F1"`..`"F12"`)
    /// - Section cycle: `"Tab"` / `"BackTab"`
    /// - Menu Up/Down: `"Up"` / `"Down"` and `'k'` / `'j'`
    /// - Activate option: `"Enter"`
    /// - Cancel prefix/modal: `"Esc"`
    /// - Numeric `1`..`99` direct jump: `true`
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::new();
    /// assert_eq!(keys.quit_key(), KeyCode::Char('q'));
    /// ```
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            quit: KeyCode::Char('q'),
            prev_page: KeyCode::Char('['),
            next_page: KeyCode::Char(']'),
            next_section: KeyCode::Tab,
            prev_section: KeyCode::BackTab,
            cursor_up: KeyCode::Up,
            cursor_down: KeyCode::Down,
            alt_up: Some(KeyCode::Char('k')),
            alt_down: Some(KeyCode::Char('j')),
            activate: KeyCode::Enter,
            cancel: KeyCode::Esc,
            numeric_jump: true,
        }
    }

    /// Sets the quit shortkey (default: `'q'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().quit('x');
    /// assert_eq!(keys.quit_key(), KeyCode::Char('x'));
    /// ```
    #[must_use]
    pub fn quit(mut self, key: impl IntoKeyCode) -> Self {
        self.quit = key.into_key_code();
        self
    }

    /// Sets the previous-page and next-page cycle shortkeys (default: `'['` and `']'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().page_cycle('<', '>');
    /// assert_eq!(keys.prev_page_key(), KeyCode::Char('<'));
    /// assert_eq!(keys.next_page_key(), KeyCode::Char('>'));
    /// ```
    #[must_use]
    pub fn page_cycle(mut self, prev: impl IntoKeyCode, next: impl IntoKeyCode) -> Self {
        self.prev_page = prev.into_key_code();
        self.next_page = next.into_key_code();
        self
    }

    /// Sets the forward and backward section focus shortkeys (default: `"Tab"` and `"BackTab"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().section_cycle("Tab", "BackTab");
    /// assert_eq!(keys.next_section_key(), KeyCode::Tab);
    /// ```
    #[must_use]
    pub fn section_cycle(mut self, next: impl IntoKeyCode, prev: impl IntoKeyCode) -> Self {
        self.next_section = next.into_key_code();
        self.prev_section = prev.into_key_code();
        self
    }

    /// Sets the primary Up and Down menu navigation keys (default: `"Up"` and `"Down"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().up_down("Up", "Down");
    /// assert_eq!(keys.cursor_up_key(), KeyCode::Up);
    /// ```
    #[must_use]
    pub fn up_down(mut self, up: impl IntoKeyCode, down: impl IntoKeyCode) -> Self {
        self.cursor_up = up.into_key_code();
        self.cursor_down = down.into_key_code();
        self
    }

    /// Sets the secondary (Vim-style) Up and Down menu navigation keys (default: `'k'` and `'j'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().vim_up_down('w', 's');
    /// assert_eq!(keys.alt_up_key(), Some(KeyCode::Char('w')));
    /// ```
    #[must_use]
    pub fn vim_up_down(mut self, up: impl IntoKeyCode, down: impl IntoKeyCode) -> Self {
        self.alt_up = Some(up.into_key_code());
        self.alt_down = Some(down.into_key_code());
        self
    }

    /// Disables secondary Vim-style Up/Down keys so only the primary `up_down` keys move the cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::KeyBindings;
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
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().activate("Space");
    /// assert_eq!(keys.activate_key(), KeyCode::Char(' '));
    /// ```
    #[must_use]
    pub fn activate(mut self, key: impl IntoKeyCode) -> Self {
        self.activate = key.into_key_code();
        self
    }

    /// Sets the cancel / back key (default: `"Esc"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::{KeyBindings, KeyCode};
    ///
    /// let keys = KeyBindings::default().cancel("Esc");
    /// assert_eq!(keys.cancel_key(), KeyCode::Esc);
    /// ```
    #[must_use]
    pub fn cancel(mut self, key: impl IntoKeyCode) -> Self {
        self.cancel = key.into_key_code();
        self
    }

    /// Enables or disables `1`..`99` direct numeric option jumping (default: `true`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::KeyBindings;
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

    /// Returns the configured quit key.
    #[must_use]
    #[inline]
    pub const fn quit_key(&self) -> KeyCode {
        self.quit
    }

    /// Returns the configured previous-page key.
    #[must_use]
    #[inline]
    pub const fn prev_page_key(&self) -> KeyCode {
        self.prev_page
    }

    /// Returns the configured next-page key.
    #[must_use]
    #[inline]
    pub const fn next_page_key(&self) -> KeyCode {
        self.next_page
    }

    /// Returns the configured forward section-cycle key.
    #[must_use]
    #[inline]
    pub const fn next_section_key(&self) -> KeyCode {
        self.next_section
    }

    /// Returns the configured backward section-cycle key.
    #[must_use]
    #[inline]
    pub const fn prev_section_key(&self) -> KeyCode {
        self.prev_section
    }

    /// Returns the primary cursor-up key.
    #[must_use]
    #[inline]
    pub const fn cursor_up_key(&self) -> KeyCode {
        self.cursor_up
    }

    /// Returns the primary cursor-down key.
    #[must_use]
    #[inline]
    pub const fn cursor_down_key(&self) -> KeyCode {
        self.cursor_down
    }

    /// Returns the optional secondary cursor-up key.
    #[must_use]
    #[inline]
    pub const fn alt_up_key(&self) -> Option<KeyCode> {
        self.alt_up
    }

    /// Returns the optional secondary cursor-down key.
    #[must_use]
    #[inline]
    pub const fn alt_down_key(&self) -> Option<KeyCode> {
        self.alt_down
    }

    /// Returns `true` if `code` matches either the primary or secondary cursor-up key.
    #[must_use]
    #[inline]
    pub fn is_up(&self, code: KeyCode) -> bool {
        code == self.cursor_up || self.alt_up == Some(code)
    }

    /// Returns `true` if `code` matches either the primary or secondary cursor-down key.
    #[must_use]
    #[inline]
    pub fn is_down(&self, code: KeyCode) -> bool {
        code == self.cursor_down || self.alt_down == Some(code)
    }

    /// Returns the configured option activation key.
    #[must_use]
    #[inline]
    pub const fn activate_key(&self) -> KeyCode {
        self.activate
    }

    /// Returns the configured cancel key.
    #[must_use]
    #[inline]
    pub const fn cancel_key(&self) -> KeyCode {
        self.cancel
    }

    /// Returns `true` if `1`..`99` numeric option jumping is enabled.
    #[must_use]
    #[inline]
    pub const fn is_numeric_jump_enabled(&self) -> bool {
        self.numeric_jump
    }
}

/// Normalized keyboard press event combining [`KeyCode`] with control/alt flags.
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::{KeyCode, KeyPress};
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
    /// use tui_rs::dashboard::{KeyCode, KeyPress};
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
    /// use tui_rs::dashboard::{KeyCode, KeyPress};
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
    /// use tui_rs::dashboard::KeyPress;
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
    /// use tui_rs::dashboard::KeyPress;
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
    /// use tui_rs::dashboard::{KeyCode, KeyPress};
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
    /// use tui_rs::dashboard::KeyPress;
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
    /// use tui_rs::dashboard::KeyPress;
    ///
    /// assert!(!KeyPress::char('a').has_alt());
    /// ```
    #[must_use]
    #[inline]
    pub const fn has_alt(&self) -> bool {
        self.alt
    }

    /// Converts a `crossterm::event::KeyEvent` into a [`KeyPress`], returning `None`
    /// for unsupported media/modifier-only keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::{KeyCode as CtKeyCode, KeyEvent, KeyModifiers};
    /// use tui_rs::dashboard::{KeyCode, KeyPress};
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
            CtKeyCode::Tab => KeyCode::Tab,
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
