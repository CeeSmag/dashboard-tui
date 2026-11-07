//! Terminal session lifecycle management, RAII guards, and panic hooks.

pub mod panic_hook;
pub mod session_guard;

pub use panic_hook::{install_panic_restoration_hook, restore_terminal_emergency};
pub use session_guard::{TerminalSessionConfig, TerminalSessionGuard};
