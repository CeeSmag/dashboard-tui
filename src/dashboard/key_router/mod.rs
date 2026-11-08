//! Keyboard input representation (`KeyPress`, `KeyCode`), event consumption states and
//! configurable shortkeys (`KeyBindings`).
//!
//! Synthesizes `gitui`'s `EventState` (`Consumed` / `NotConsumed`) with
//! ergonomic conversion from `crossterm::event::KeyEvent`.

mod bindings;
mod events;
mod key_code;
mod key_press;

pub use bindings::KeyBindings;
pub use events::{EngineAction, EventState};
pub use key_code::{IntoKeyCode, KeyCode};
pub use key_press::{IntoKeyPress, KeyPress};
