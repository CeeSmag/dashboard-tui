//! The 3-method `DashboardBackend` contract separating Tier 1 from client domains.
//!
//! Directly ports `Backend` from `ref/dashboard/de_dashboard.go`. The Tier 1
//! [`DashboardEngine`](crate::dashboard::DashboardEngine) knows nothing about
//! domain specifics (such as FUSE mounts, rclone remotes, or systemd timers);
//! it queries the backend exclusively through this minimal trait while Tier 3
//! add-ons access domain-specific methods on `B` directly.

use crate::core::color::Color;
use crate::core::priority_strip::PrioritySegment;
use crate::error::TuiError;

/// Minimal contract between the Tier 1 [`DashboardEngine`](crate::dashboard::DashboardEngine)
/// and the host application's domain backend.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment, Style};
/// use tui_rs::dashboard::DashboardBackend;
///
/// struct MockVaultBackend;
///
/// impl DashboardBackend for MockVaultBackend {
///     fn app_title(&self) -> &str {
///         "VaultDrive"
///     }
///
///     fn status_badge(&self) -> (&str, Color) {
///         ("MOUNTED", Color::Green)
///     }
///
///     fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
///         vec![PrioritySegment::new("▲ 1.2 MB/s ▼ 4.5 MB/s", Style::DEFAULT, 100)]
///     }
/// }
///
/// let backend = MockVaultBackend;
/// assert_eq!(backend.app_title(), "VaultDrive");
/// ```
pub trait DashboardBackend: 'static {
    /// Returns the short application name displayed on the left of the top header bar.
    fn app_title(&self) -> &str;

    /// Returns the primary status badge text and its semantic foreground [`Color`].
    fn status_badge(&self) -> (&str, Color);

    /// Returns priority-ranked telemetry segments for the top header bar.
    ///
    /// Segments with higher `priority` numbers are kept longest when
    /// the terminal width shrinks.
    fn header_segments(&self) -> Vec<PrioritySegment<'_>>;

    /// Called periodically on background ticks so the backend can refresh cached
    /// metrics if desired. Defaults to a no-op.
    fn on_tick(&mut self) {}

    /// Persists user interface preferences (such as theme or spacing gaps) to disk.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if saving preferences fails.
    fn save_ui_prefs(&mut self) -> Result<(), TuiError> {
        Ok(())
    }
}
