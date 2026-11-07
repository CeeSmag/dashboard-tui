//! Universal `[ Yes ]` / `[ No ]` confirmation dialog (`ConfirmModal`).
//!
//! Directly ports `ConfirmModal` from `ref/dashboard/addons/universal/prompts.go`.

use crate::core::buffer::Buffer;
use crate::core::grapheme::wrap_lines_iter;
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};

/// Callback invoked when the user confirms a [`ConfirmModal`].
pub type ConfirmActionFn<B> = Box<dyn FnMut(&mut B) -> Result<String, String>>;

/// Yes/No confirmation modal dialog.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::DashboardBackend;
/// use tui_rs::universal::ConfirmModal;
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let modal = ConfirmModal::<App>::new(
///     "Unmount Vault",
///     "Are you sure you want to unmount the active FUSE filesystem?",
///     |_b| Ok("Vault unmounted".to_string()),
/// );
/// assert!(!modal.is_yes_selected());
/// ```
pub struct ConfirmModal<B: DashboardBackend> {
    title: String,
    question: String,
    yes_selected: bool,
    on_confirm: ConfirmActionFn<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for ConfirmModal<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConfirmModal")
            .field("title", &self.title)
            .field("yes_selected", &self.yes_selected)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> ConfirmModal<B> {
    /// Creates a new [`ConfirmModal`] defaulting to `[ No ]` selected for safety.
    #[must_use]
    pub fn new<F>(title: impl Into<String>, question: impl Into<String>, on_confirm: F) -> Self
    where
        F: FnMut(&mut B) -> Result<String, String> + 'static,
    {
        Self {
            title: title.into(),
            question: question.into(),
            yes_selected: false,
            on_confirm: Box::new(on_confirm),
        }
    }

    /// Configures whether `[ Yes ]` is initially focused instead of `[ No ]`.
    #[must_use]
    #[inline]
    pub const fn with_default_yes(mut self, yes: bool) -> Self {
        self.yes_selected = yes;
        self
    }

    /// Returns `true` if `[ Yes ]` is currently highlighted.
    #[must_use]
    #[inline]
    pub const fn is_yes_selected(&self) -> bool {
        self.yes_selected
    }

    fn execute_confirm(&mut self, backend: &mut B) -> ModalResult {
        match (self.on_confirm)(backend) {
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
            Err(err) => ModalResult::CloseWithToast {
                message: err,
                is_error: true,
            },
        }
    }
}

impl<B: DashboardBackend> Modal<B> for ConfirmModal<B> {
    fn title(&self) -> &str {
        &self.title
    }

    fn subtitle(&self) -> &'static str {
        "←→/Tab toggle • y/n • Enter confirm • Esc cancel"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let w = (viewport.width().get() * 3 / 5).clamp(38, 60);
        (TerminalWidth::new(w), TerminalHeight::new(8))
    }

    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, _tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let max_w = inner.width().get();
        let mut cur_y = inner.y();

        for line in wrap_lines_iter(&self.question, TerminalWidth::new(max_w)).take(3) {
            let _ = buffer.set_string(
                inner.x(),
                cur_y,
                line,
                Style::new().with_fg(palette.text),
                max_w,
            );
            cur_y = cur_y.saturating_add(1);
        }

        let btn_y = inner.bottom().saturating_sub(1).max(cur_y);
        let yes_style = if self.yes_selected {
            Style::new()
                .with_fg(palette.accent2)
                .with_bg(palette.highlight_bg)
                .bold()
        } else {
            Style::new().with_fg(palette.muted)
        };
        let no_style = if self.yes_selected {
            Style::new().with_fg(palette.muted)
        } else {
            Style::new()
                .with_fg(palette.accent2)
                .with_bg(palette.highlight_bg)
                .bold()
        };

        let yes_str = if self.yes_selected {
            " ❯ [ Yes ] "
        } else {
            "   [ Yes ] "
        };
        let no_str = if self.yes_selected {
            "   [ No ] "
        } else {
            " ❯ [ No ] "
        };

        let mut cx = buffer.set_string(inner.x(), btn_y, yes_str, yes_style, max_w);
        cx = cx.saturating_add(3);
        if cx.get() < inner.right().get() {
            let _ = buffer.set_string(cx, btn_y, no_str, no_style, inner.right().get() - cx.get());
        }
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        match key.code() {
            KeyCode::Esc | KeyCode::Char('n' | 'N') => ModalResult::Close,
            KeyCode::Char('y' | 'Y') => self.execute_confirm(ctx.backend),
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Char('h' | 'l') => {
                self.yes_selected = !self.yes_selected;
                ModalResult::Continue
            }
            KeyCode::Enter => {
                if self.yes_selected {
                    self.execute_confirm(ctx.backend)
                } else {
                    ModalResult::Close
                }
            }
            _ => ModalResult::Continue,
        }
    }
}
