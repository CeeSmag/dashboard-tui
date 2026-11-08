//! [`KeyBindings`]: configurable global shortkeys.

use super::key_code::KeyCode;
use super::key_press::{IntoKeyPress, KeyPress};

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
    quit_popup_title: &'static str,
    quit_popup_description: &'static str,
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
            quit: KeyPress::char('q').with_ctrl(true),
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
            quit_popup_title: "Quit",
            quit_popup_description: "Are you sure you want to quit?",
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
    /// `q` / `Esc` cancels (default: `false`, quit immediately). `Ctrl+C` always leaves at once ([`EngineAction::Back`](super::events::EngineAction::Back)).
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

    /// Sets the title and description shown in the quit warning popup
    /// (defaults: `"Quit"` / `"Are you sure you want to quit?"`). Only used when
    /// [`confirm_quit`](Self::confirm_quit) is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::dashboard::KeyBindings;
    ///
    /// let keys = KeyBindings::default()
    ///     .confirm_quit(true)
    ///     .quit_popup("Stop server", "This stops all running services.");
    /// assert_eq!(keys.quit_popup_title(), "Stop server");
    /// assert_eq!(keys.quit_popup_description(), "This stops all running services.");
    /// ```
    #[must_use]
    #[inline]
    pub const fn quit_popup(mut self, title: &'static str, description: &'static str) -> Self {
        self.quit_popup_title = title;
        self.quit_popup_description = description;
        self
    }

    /// Returns the title of the quit warning popup.
    #[must_use]
    #[inline]
    pub const fn quit_popup_title(&self) -> &'static str {
        self.quit_popup_title
    }

    /// Returns the description (question text) of the quit warning popup.
    #[must_use]
    #[inline]
    pub const fn quit_popup_description(&self) -> &'static str {
        self.quit_popup_description
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
