//! Multi-digit numeric shortcut accumulator (`PrefixJumpState`) for `1`..`99` option jumping.
//!
//! Directly ports `dispatchDigit`, `commitDigitBuf`, and `maxOptionNumberOnPage`
//! from `ref/dashboard/de_dashboard.go`:
//! - If the active page's highest option number is `<= 9` (or the first digit `d`
//!   cannot form a valid 2-digit prefix because `d * 10 > max_option`), pressing `d`
//!   fires `[d]` **immediately** with zero delay.
//! - If `d * 10 <= max_option` (e.g., pressing `'1'` or `'2'` on a page with options
//!   up to `[20]`), the first digit is buffered for up to `450ms`:
//!   - Pressing a second digit `0..9` within `450ms` immediately fires `d * 10 + d2`.
//!   - Pressing `Enter` within `450ms` immediately commits the single digit `d`.
//!   - Pressing `Esc` cancels the buffered digit.
//!   - If `450ms` elapses without a second digit, `poll_expired` commits the single digit `d`.

use std::time::{Duration, Instant};

/// Timeout window (`450ms`) for entering the second digit of a 2-digit option number.
pub const MULTI_DIGIT_TIMEOUT: Duration = Duration::from_millis(450);

/// Result of feeding a digit key into [`PrefixJumpState`].
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::DigitDispatch;
///
/// assert_eq!(DigitDispatch::FireImmediate(5), DigitDispatch::FireImmediate(5));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitDispatch {
    /// Immediately execute the option with this number (`1..=99`).
    FireImmediate(u16),
    /// Buffered the first digit (`1..=9`); waiting up to `450ms` for a second digit.
    BufferedFirstDigit(u16),
    /// Digit was ignored (e.g., leading `'0'` when no prefix is buffered).
    Ignored,
}

/// Tracks multi-digit numeric shortcut input (`1`..`99`) with a `450ms` timeout.
///
/// # Examples
///
/// ```
/// use std::time::Instant;
/// use tui_rs::dashboard::{DigitDispatch, PrefixJumpState};
///
/// let mut jump = PrefixJumpState::new();
/// let now = Instant::now();
/// // On a page with max_option = 20, pressing '1' buffers the first digit:
/// assert_eq!(jump.feed_digit(1, 20, now), DigitDispatch::BufferedFirstDigit(1));
/// // Pressing '9' immediately fires option [19]:
/// assert_eq!(jump.feed_digit(9, 20, now), DigitDispatch::FireImmediate(19));
/// assert!(!jump.has_pending());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct PrefixJumpState {
    pending_digit: Option<(u16, Instant)>,
}

impl PrefixJumpState {
    /// Creates an empty [`PrefixJumpState`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::PrefixJumpState;
    ///
    /// let state = PrefixJumpState::new();
    /// assert!(!state.has_pending());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            pending_digit: None,
        }
    }

    /// Returns `true` if a first digit is currently buffered waiting for a second digit.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::dashboard::PrefixJumpState;
    ///
    /// assert!(!PrefixJumpState::new().has_pending());
    /// ```
    #[must_use]
    #[inline]
    pub const fn has_pending(&self) -> bool {
        self.pending_digit.is_some()
    }

    /// Returns the currently buffered first digit (`1..=9`), if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::dashboard::PrefixJumpState;
    ///
    /// let mut state = PrefixJumpState::new();
    /// state.feed_digit(2, 20, Instant::now());
    /// assert_eq!(state.pending_digit(), Some(2));
    /// ```
    #[must_use]
    #[inline]
    pub const fn pending_digit(&self) -> Option<u16> {
        match self.pending_digit {
            Some((d, _)) => Some(d),
            None => None,
        }
    }

    /// Cancels any buffered digit.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::dashboard::PrefixJumpState;
    ///
    /// let mut state = PrefixJumpState::new();
    /// state.feed_digit(1, 20, Instant::now());
    /// state.clear();
    /// assert!(!state.has_pending());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.pending_digit = None;
    }

    /// Immediately commits and returns the buffered single digit (e.g., when the user
    /// presses `Enter` before the `450ms` window expires).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::dashboard::PrefixJumpState;
    ///
    /// let mut state = PrefixJumpState::new();
    /// state.feed_digit(1, 20, Instant::now());
    /// assert_eq!(state.commit_now(), Some(1));
    /// assert!(!state.has_pending());
    /// ```
    #[inline]
    pub fn commit_now(&mut self) -> Option<u16> {
        self.pending_digit.take().map(|(d, _)| d)
    }

    /// Checks whether the buffered digit's `450ms` window has elapsed at `now`,
    /// returning `Some(digit)` if it should now fire as a single-digit option.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// use tui_rs::dashboard::PrefixJumpState;
    ///
    /// let mut state = PrefixJumpState::new();
    /// let start = Instant::now();
    /// state.feed_digit(1, 20, start);
    /// assert_eq!(state.poll_expired(start + Duration::from_millis(100)), None);
    /// assert_eq!(state.poll_expired(start + Duration::from_millis(500)), Some(1));
    /// ```
    #[inline]
    pub fn poll_expired(&mut self, now: Instant) -> Option<u16> {
        if let Some((d, stamped)) = self.pending_digit {
            if now.saturating_duration_since(stamped) >= MULTI_DIGIT_TIMEOUT {
                self.pending_digit = None;
                return Some(d);
            }
        }
        None
    }

    /// Processes a numeric digit `digit` (`0..=9`) given the active page's
    /// `max_option_number`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::dashboard::{DigitDispatch, PrefixJumpState};
    ///
    /// let mut state = PrefixJumpState::new();
    /// let now = Instant::now();
    /// // On a page where max_option_number = 20, '3' (30 > 20) fires immediately!
    /// assert_eq!(state.feed_digit(3, 20, now), DigitDispatch::FireImmediate(3));
    /// ```
    pub fn feed_digit(
        &mut self,
        digit: u16,
        max_option_number: u16,
        now: Instant,
    ) -> DigitDispatch {
        if digit > 9 {
            return DigitDispatch::Ignored;
        }
        if let Some((first, stamped)) = self.pending_digit.take() {
            if now.saturating_duration_since(stamped) < MULTI_DIGIT_TIMEOUT {
                let combined = first.saturating_mul(10).saturating_add(digit);
                return DigitDispatch::FireImmediate(combined);
            }
            // Previous digit expired; fall through to process `digit` fresh
        }

        if digit == 0 {
            return DigitDispatch::Ignored;
        }

        // If a 2-digit option starting with `digit` cannot exist on this page,
        // fire immediately with zero latency!
        if max_option_number <= 9 || digit.saturating_mul(10) > max_option_number {
            return DigitDispatch::FireImmediate(digit);
        }

        self.pending_digit = Some((digit, now));
        DigitDispatch::BufferedFirstDigit(digit)
    }
}
