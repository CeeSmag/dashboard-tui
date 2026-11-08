//! Composable panic restoration hook for terminal state recovery.
//!
//! Installing this panic hook ensures that if any thread panics while the
//! terminal is in raw mode or the alternate screen buffer, canonical terminal
//! state is restored *before* the standard panic backtrace is printed to
//! `stderr`, preventing staircase output or an invisible cursor.

use std::io::Write;

/// Installs a composable panic hook that restores canonical terminal state
/// before delegating to the previously registered panic hook.
///
/// Specifically, this hook:
/// 1. Disables raw mode via `crossterm::terminal::disable_raw_mode()`.
/// 2. Writes raw ANSI escape sequences to reset DEC Mode 2026 (synchronized output),
///    DEC Mode 2027 (grapheme clustering), DEC Mode 1007 (alternate scroll),
///    OSC `9;4` progress bar state, exits the alternate screen (`?1049l`),
///    shows the cursor (`?25h`), and resets SGR attributes (`\x1b[0m`).
/// 3. Invokes the previous panic hook so standard diagnostics are preserved.
///
/// # Examples
///
/// ```
/// use dashboard_tui::terminal::install_panic_restoration_hook;
///
/// // Safe to call at application startup before entering a TerminalSessionGuard:
/// install_panic_restoration_hook();
/// ```
pub fn install_panic_restoration_hook() {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal_emergency();
        previous_hook(panic_info);
    }));
}

/// Performs best-effort emergency restoration of terminal state.
///
/// Safe to call from signal handlers or panic hooks. Ignores I/O errors.
///
/// # Examples
///
/// ```
/// use dashboard_tui::terminal::restore_terminal_emergency;
///
/// restore_terminal_emergency();
/// ```
pub fn restore_terminal_emergency() {
    let _ = crossterm::terminal::disable_raw_mode();
    let mut stderr = std::io::stderr();
    // Reset DEC 2026 sync, DEC 2027 grapheme, DEC 1007 alt scroll, OSC 9;4 progress,
    // leave alternate screen (?1049l), show cursor (?25h), reset SGR (0m).
    let _ = stderr
        .write_all(b"\x1b[?2026l\x1b[?2027l\x1b[?1007l\x1b]9;4;0;0\x07\x1b[?1049l\x1b[?25h\x1b[0m");
    let _ = stderr.flush();
}
