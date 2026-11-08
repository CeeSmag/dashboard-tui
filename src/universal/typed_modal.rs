//! Destructive action guard modal (`TypedModal`) requiring an exact confirmation phrase.
//!
//! Directly ports `TypedModal` from `ref/dashboard/addons/universal/prompts.go`.

use crate::core::buffer::Buffer;
use crate::core::grapheme::wrap_lines_iter;
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::universal::confirm_modal::ConfirmActionFn;
use crate::widgets::text_input::{TextInput, TextInputState};

/// Destructive action confirmation modal requiring the user to type an exact phrase
/// (e.g., `"PURGE"` or `"RESET"`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::DashboardBackend;
/// use dashboard_tui::universal::TypedModal;
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let modal = TypedModal::<App>::new(
///     "Purge Local Cache",
///     "This will delete all cached chunks.",
///     "PURGE",
///     |_b| Ok("Cache purged".to_string()),
/// );
/// assert_eq!(modal.expected_phrase(), "PURGE");
/// ```
pub struct TypedModal<B: DashboardBackend> {
    title: String,
    warning: String,
    expected_phrase: String,
    input: TextInputState,
    error_message: String,
    on_confirm: ConfirmActionFn<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for TypedModal<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TypedModal")
            .field("title", &self.title)
            .field("expected_phrase", &self.expected_phrase)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> TypedModal<B> {
    /// Creates a new [`TypedModal`] requiring `expected_phrase` to be typed.
    #[must_use]
    pub fn new<F>(
        title: impl Into<String>,
        warning: impl Into<String>,
        expected_phrase: impl Into<String>,
        on_confirm: F,
    ) -> Self
    where
        F: FnMut(&mut B) -> Result<String, String> + 'static,
    {
        Self {
            title: title.into(),
            warning: warning.into(),
            expected_phrase: expected_phrase.into(),
            input: TextInputState::new(),
            error_message: String::new(),
            on_confirm: Box::new(on_confirm),
        }
    }

    /// Returns the required confirmation phrase.
    #[must_use]
    #[inline]
    pub fn expected_phrase(&self) -> &str {
        &self.expected_phrase
    }
}

impl<B: DashboardBackend> Modal<B> for TypedModal<B> {
    fn title(&self) -> &str {
        &self.title
    }

    fn subtitle(&self) -> &'static str {
        "Type phrase + Enter confirm • Esc cancel"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let w = (viewport.width().get() * 2 / 3).clamp(44, 68);
        (TerminalWidth::new(w), TerminalHeight::new(11))
    }

    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let max_w = inner.width().get();
        let mut cur_y = inner.y();

        for line in wrap_lines_iter(&self.warning, TerminalWidth::new(max_w)).take(2) {
            let _ = buffer.set_string(
                inner.x(),
                cur_y,
                line,
                Style::new().with_fg(palette.warn),
                max_w,
            );
            cur_y = cur_y.saturating_add(1);
        }

        let prompt_line = format!("Type \"{}\" to confirm:", self.expected_phrase);
        let _ = buffer.set_string(
            inner.x(),
            cur_y,
            &prompt_line,
            Style::new().with_fg(palette.text).bold(),
            max_w,
        );
        cur_y = cur_y.saturating_add(1);

        if cur_y.get().saturating_add(3) <= inner.bottom().get() {
            TextInput::new(&self.input).with_tty_mode(tty_mode).render(
                buffer,
                inner.x(),
                cur_y,
                max_w,
                palette,
            );
            cur_y = cur_y.saturating_add(3);
        }

        if !self.error_message.is_empty() && cur_y.get() < inner.bottom().get() {
            let _ = buffer.set_string(
                inner.x(),
                cur_y,
                &self.error_message,
                Style::new().with_fg(palette.err).bold(),
                max_w,
            );
        }
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        match key.code() {
            KeyCode::Esc => ModalResult::Close,
            KeyCode::Enter => {
                if self.input.value() != self.expected_phrase {
                    self.error_message = format!("Must match \"{}\" exactly", self.expected_phrase);
                    return ModalResult::Continue;
                }
                match (self.on_confirm)(ctx.backend) {
                    Ok(msg) => ModalResult::CloseWithToast {
                        message: msg,
                        is_error: false,
                    },
                    Err(err) => ModalResult::CloseWithToast {
                        message: err,
                        is_error: true,
                    },
                }
            }
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
            KeyCode::Char(ch) if !key.has_ctrl() && !key.has_alt() => {
                self.error_message.clear();
                self.input.insert_char(ch);
                ModalResult::Consumed
            }
            _ => ModalResult::Continue,
        }
    }
}
