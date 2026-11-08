//! Event consumption state ([`EventState`]) and the host-facing [`EngineAction`].

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
