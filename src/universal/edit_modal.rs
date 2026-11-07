//! Universal single-line text and masked password input modal (`EditModal`).
//!
//! Directly ports `EditModal` from `ref/dashboard/addons/universal/prompts.go`.

use crate::core::buffer::Buffer;
use crate::core::grapheme::wrap_lines_iter;
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::widgets::text_input::{TextInput, TextInputState};

/// Callback invoked when the user presses `Enter` in an [`EditModal`].
///
/// Returning `Ok(toast_msg)` closes the modal and displays `toast_msg`;
/// returning `Err(err_msg)` keeps the modal open and displays the inline validation error.
pub type EditSubmitFn<B> = Box<dyn FnMut(&mut B, &str) -> Result<String, String>>;

/// Single-line text or masked password input modal with inline validation.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::DashboardBackend;
/// use tui_rs::universal::EditModal;
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let modal = EditModal::<App>::new(
///     "Edit Remote Name",
///     "Enter the rclone remote identifier:",
///     "gdrive:vault",
///     |_backend, val| {
///         if val.is_empty() {
///             Err("Remote name cannot be empty".to_string())
///         } else {
///             Ok(format!("Updated remote to {val}"))
///         }
///     },
/// );
/// assert_eq!(modal.value(), "gdrive:vault");
/// ```
pub struct EditModal<B: DashboardBackend> {
    title: String,
    prompt: String,
    input: TextInputState,
    masked: bool,
    error_message: String,
    on_submit: EditSubmitFn<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for EditModal<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EditModal")
            .field("title", &self.title)
            .field("masked", &self.masked)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> EditModal<B> {
    /// Creates a new [`EditModal`].
    #[must_use]
    pub fn new<F>(
        title: impl Into<String>,
        prompt: impl Into<String>,
        initial: impl Into<String>,
        on_submit: F,
    ) -> Self
    where
        F: FnMut(&mut B, &str) -> Result<String, String> + 'static,
    {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            input: TextInputState::from_text(initial),
            masked: false,
            error_message: String::new(),
            on_submit: Box::new(on_submit),
        }
    }

    /// Enables masked secret mode (`'•'`) for passwords or passphrases.
    #[must_use]
    #[inline]
    pub const fn with_masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Returns the current edited string slice.
    #[must_use]
    #[inline]
    pub fn value(&self) -> &str {
        self.input.value()
    }
}

impl<B: DashboardBackend> Modal<B> for EditModal<B> {
    fn title(&self) -> &str {
        &self.title
    }

    fn subtitle(&self) -> &'static str {
        "Enter save • Ctrl+U clear • Esc cancel"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let w = (viewport.width().get() * 2 / 3).clamp(42, 68);
        (TerminalWidth::new(w), TerminalHeight::new(10))
    }

    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let max_w = inner.width().get();
        let mut cur_y = inner.y();

        for line in wrap_lines_iter(&self.prompt, TerminalWidth::new(max_w)).take(2) {
            let _ = buffer.set_string(
                inner.x(),
                cur_y,
                line,
                Style::new().with_fg(palette.muted),
                max_w,
            );
            cur_y = cur_y.saturating_add(1);
        }

        cur_y = cur_y.saturating_add(1);
        if cur_y.get().saturating_add(3) <= inner.bottom().get() {
            TextInput::new(&self.input)
                .with_masked(self.masked)
                .with_tty_mode(tty_mode)
                .render(buffer, inner.x(), cur_y, max_w, palette);
            cur_y = cur_y.saturating_add(3);
        }

        if !self.error_message.is_empty() && cur_y.get() < inner.bottom().get() {
            let err_style = Style::new().with_fg(palette.err).bold();
            let prefix = if tty_mode { "[!] " } else { "✗ " };
            let cx = buffer.set_string(inner.x(), cur_y, prefix, err_style, max_w);
            let _ = buffer.set_string(
                cx,
                cur_y,
                &self.error_message,
                err_style,
                inner.right().get().saturating_sub(cx.get()),
            );
        }
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        match key.code() {
            KeyCode::Esc => ModalResult::Close,
            KeyCode::Enter => match (self.on_submit)(ctx.backend, self.input.value()) {
                Ok(msg) => {
                    if msg.is_empty() {
                        ModalResult::Close
                    } else {
                        ModalResult::CloseWithToast {
                            message: msg,
                            is_error: false,
                        }
                    }
                }
                Err(err) => {
                    self.error_message = err;
                    ModalResult::Continue
                }
            },
            KeyCode::Backspace => {
                self.error_message.clear();
                self.input.backspace();
                ModalResult::Continue
            }
            KeyCode::Delete => {
                self.error_message.clear();
                self.input.delete();
                ModalResult::Continue
            }
            KeyCode::Left => {
                self.input.move_left();
                ModalResult::Continue
            }
            KeyCode::Right => {
                self.input.move_right();
                ModalResult::Continue
            }
            KeyCode::Home => {
                self.input.move_home();
                ModalResult::Continue
            }
            KeyCode::End => {
                self.input.move_end();
                ModalResult::Continue
            }
            KeyCode::Char('u') if key.has_ctrl() => {
                self.error_message.clear();
                self.input.clear();
                ModalResult::Continue
            }
            KeyCode::Char(ch) if !key.has_ctrl() && !key.has_alt() => {
                self.error_message.clear();
                self.input.insert_char(ch);
                ModalResult::Continue
            }
            _ => ModalResult::Continue,
        }
    }
}
