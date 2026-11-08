//! Zero-allocation byte/rate formatters and Braille spinner frames.
//!
//! Produces [`StackStr`] values on the stack without heap allocations during
//! frame rendering. Matches `FmtBytes`, `FmtRate`, `FmtRateShort`, and `Spin`
//! from `ref/dashboard/de_widgets.go` and `de_monitors.go`.

use core::fmt::Write;

use crate::core::buffer::StackStr;

/// 10-frame Braille spinner animation sequence.
const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Returns the Braille spinner glyph for a monotonic frame counter `tick`.
///
/// # Examples
///
/// ```
/// use dashboard_tui::components::spinner_frame;
///
/// assert_eq!(spinner_frame(0), "⠋");
/// assert_eq!(spinner_frame(10), "⠋");
/// ```
#[must_use]
#[inline]
pub const fn spinner_frame(tick: usize) -> &'static str {
    SPINNER_FRAMES[tick % SPINNER_FRAMES.len()]
}

/// Formats a byte count into a human-readable IEC/SI string (`"512 B"`, `"4.2 KB"`, `"1.5 GB"`)
/// on the stack without heap allocation.
///
/// # Examples
///
/// ```
/// use dashboard_tui::components::format_bytes;
///
/// assert_eq!(format_bytes(0).as_str(), "0 B");
/// assert_eq!(format_bytes(512).as_str(), "512 B");
/// assert_eq!(format_bytes(1024 * 1024).as_str(), "1.0 MB");
/// ```
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "u64 to f64 for display formatting"
)]
pub fn format_bytes(bytes: u64) -> StackStr<32> {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    let mut out = StackStr::<32>::new();
    if bytes >= TB {
        let _ = write!(out, "{:.2} TB", (bytes as f64) / (TB as f64));
    } else if bytes >= GB {
        let _ = write!(out, "{:.1} GB", (bytes as f64) / (GB as f64));
    } else if bytes >= MB {
        let _ = write!(out, "{:.1} MB", (bytes as f64) / (MB as f64));
    } else if bytes >= KB {
        let _ = write!(out, "{:.1} KB", (bytes as f64) / (KB as f64));
    } else {
        let _ = write!(out, "{bytes} B");
    }
    out
}

/// Formats a throughput rate (`bytes_per_sec`) into a human-readable string
/// (`"0 B/s"`, `"512 KB/s"`, `"12.4 MB/s"`) on the stack.
///
/// # Examples
///
/// ```
/// use dashboard_tui::components::format_rate;
///
/// assert_eq!(format_rate(0.0).as_str(), "0 B/s");
/// assert_eq!(format_rate(1024.0 * 500.0).as_str(), "500 KB/s");
/// ```
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped positive values for display"
)]
pub fn format_rate(bytes_per_sec: f64) -> StackStr<32> {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let mut out = StackStr::<32>::new();
    if bytes_per_sec <= 0.0 || !bytes_per_sec.is_finite() {
        out.push_str("0 B/s");
    } else if bytes_per_sec >= GB {
        let _ = write!(out, "{:.2} GB/s", bytes_per_sec / GB);
    } else if bytes_per_sec >= MB {
        let _ = write!(out, "{:.1} MB/s", bytes_per_sec / MB);
    } else if bytes_per_sec >= KB {
        let _ = write!(out, "{} KB/s", (bytes_per_sec / KB).round() as u64);
    } else {
        let _ = write!(out, "{} B/s", bytes_per_sec.round() as u64);
    }
    out
}

/// Formats a compact axis scale ceiling label (`"≤256K"`, `"≤1.5M"`, `"≤1.0G"`)
/// for waveform center dividers on the stack.
///
/// # Examples
///
/// ```
/// use dashboard_tui::components::format_rate_short;
///
/// assert_eq!(format_rate_short(256.0 * 1024.0).as_str(), "≤256K");
/// assert_eq!(format_rate_short(10.0 * 1024.0 * 1024.0).as_str(), "≤10M");
/// ```
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped positive values for display"
)]
pub fn format_rate_short(bytes_per_sec: f64) -> StackStr<24> {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let mut out = StackStr::<24>::new();
    if bytes_per_sec >= GB {
        let _ = write!(out, "≤{:.1}G", bytes_per_sec / GB);
    } else if bytes_per_sec >= MB {
        let v = bytes_per_sec / MB;
        if v >= 10.0 {
            let _ = write!(out, "≤{}M", v.round() as u64);
        } else {
            let _ = write!(out, "≤{v:.1}M");
        }
    } else if bytes_per_sec >= KB {
        let _ = write!(out, "≤{}K", (bytes_per_sec / KB).round() as u64);
    } else {
        let _ = write!(out, "≤{}B", bytes_per_sec.max(0.0).round() as u64);
    }
    out
}
