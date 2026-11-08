//! Universal terminal background picker (`TerminalBackgroundModal`, `terminal_background_option`).
//!
//! Lets the end user choose between keeping the terminal application's own background and
//! letting the active theme paint it. Previews live; `Esc` reverts.

use crate::core::buffer::Buffer;
use crate::core::palette::{ColorPalette, TerminalBackground};
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::dashboard::section::{OptionOutcome, OptionSpec};
use crate::widgets::choice_row::ChoiceRow;

const CHOICES: [(&str, &str, TerminalBackground); 2] = [
    (
        "Terminal default",
        "Keep the terminal application's own background (and transparency)",
        TerminalBackground::Inherit,
    ),
    (
        "Follow theme",
        "Paint the active theme's background behind the whole UI",
        TerminalBackground::Theme,
    ),
];

/// Two-choice picker for [`TerminalBackground`] with live preview.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::TerminalBackground;
/// use dashboard_tui::universal::TerminalBackgroundModal;
///
/// let modal = TerminalBackgroundModal::new(TerminalBackground::Theme);
/// assert_eq!(modal.cursor(), 1);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TerminalBackgroundModal {
    initial: TerminalBackground,
    cursor: usize,
}

impl TerminalBackgroundModal {
    /// Creates a picker starting on the `current` mode (a `Fixed` color counts as "Follow theme").
    #[must_use]
    pub const fn new(current: TerminalBackground) -> Self {
        let cursor = match current {
            TerminalBackground::Inherit => 0,
            _ => 1,
        };
        Self {
            initial: current,
            cursor,
        }
    }

    /// Returns the highlighted choice index.
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    fn selected_mode(&self) -> TerminalBackground {
        CHOICES
            .get(self.cursor)
            .map_or(TerminalBackground::Inherit, |choice| choice.2)
    }
}

impl<B: DashboardBackend> Modal<B> for TerminalBackgroundModal {
    fn title(&self) -> &'static str {
        "Terminal Background"
    }

    fn subtitle(&self) -> &'static str {
        "↑↓ preview • Enter apply • Esc revert"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let width = (viewport.width().get() * 3 / 4).clamp(46, 72);
        (TerminalWidth::new(width), TerminalHeight::new(8))
    }

    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        let mut row = inner.y();
        for (index, (label, detail, mode)) in CHOICES.iter().enumerate() {
            if row.get() >= inner.bottom().get() {
                break;
            }
            let badge = if *mode == self.initial {
                "● CURRENT"
            } else {
                ""
            };
            let number = u16::try_from(index.saturating_add(1)).unwrap_or(u16::MAX);
            let used = ChoiceRow::new(number, label)
                .with_badge(badge)
                .with_badge_color(palette.accent)
                .with_detail(detail)
                .with_selected(index == self.cursor)
                .with_tty_mode(tty_mode)
                .render(buffer, inner.x(), row, inner.width().get(), palette);
            row = row.saturating_add(used);
        }
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        match key.code() {
            KeyCode::Esc | KeyCode::Char('q') => {
                *ctx.terminal_background = self.initial;
                ModalResult::Close
            }
            KeyCode::Enter => {
                *ctx.terminal_background = self.selected_mode();
                let _ = ctx.backend.save_ui_prefs();
                ModalResult::CloseWithToast {
                    message: format!("Terminal background: {}", CHOICES[self.cursor].0),
                    is_error: false,
                }
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Char('k' | 'j') => {
                self.cursor = (self.cursor + 1) % CHOICES.len();
                *ctx.terminal_background = self.selected_mode();
                ModalResult::Continue
            }
            KeyCode::Char(ch @ '1'..='2') => {
                self.cursor = usize::from(u8::try_from(ch).unwrap_or(b'1') - b'1');
                *ctx.terminal_background = self.selected_mode();
                ModalResult::Continue
            }
            _ => ModalResult::Continue,
        }
    }
}

/// Ready-made menu option `[number] Terminal Background` opening a [`TerminalBackgroundModal`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::DashboardBackend;
/// use dashboard_tui::universal::terminal_background_option;
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let option = terminal_background_option::<App>(21);
/// assert_eq!(option.number(), 21);
/// ```
#[must_use]
pub fn terminal_background_option<B: DashboardBackend>(number: u16) -> OptionSpec<B> {
    OptionSpec::new(number, "Terminal Background", |ctx| {
        OptionOutcome::OpenModal(Box::new(TerminalBackgroundModal::new(
            *ctx.terminal_background,
        )))
    })
}
