//! Universal 4-layer spacing adjuster (`SectionGapModal`, `section_gap_option`).
//!
//! Directly ports `SectionGapModal` (`[19] Section Spacing Gaps`) from
//! `ref/dashboard/addons/universal/prompts.go`. Live-updates [`SpacingConfig`]
//! on every key press so the underlying page geometry reflows in real time
//! behind the dimmed modal backdrop.

use core::fmt::Write;

use crate::components::slider::SliderBar;
use crate::core::buffer::{Buffer, StackStr};
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::dashboard::section::{OptionOutcome, OptionSpec};
use crate::dashboard::spacing::{SpacingConfig, SpacingLayer};
use crate::widgets::choice_row::ChoiceRow;

/// Interactive 4-layer spacing adjuster modal (`[19] Section Spacing Gaps`).
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::SpacingConfig;
/// use tui_rs::universal::SectionGapModal;
///
/// let modal = SectionGapModal::new(SpacingConfig::default());
/// assert_eq!(modal.cursor(), 0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SectionGapModal {
    spacing: SpacingConfig,
    cursor: usize,
}

impl SectionGapModal {
    /// Creates a new [`SectionGapModal`] initialized with `current_spacing`.
    #[must_use]
    #[inline]
    pub const fn new(current_spacing: SpacingConfig) -> Self {
        Self {
            spacing: current_spacing,
            cursor: 0,
        }
    }

    /// Returns the currently focused layer index (`0..=3`).
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }
}

impl<B: DashboardBackend> Modal<B> for SectionGapModal {
    fn title(&self) -> &'static str {
        "Section Spacing Gaps (4-Layer)"
    }

    fn subtitle(&self) -> &'static str {
        "↑↓ layer • ←→/0-9 adjust • r reset • Enter/Esc done"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let w = (viewport.width().get() * 3 / 4).clamp(52, 74);
        (TerminalWidth::new(w), TerminalHeight::new(16))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "layer indices and values are small u16 constants"
    )]
    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let max_w = inner.width().get();
        let mut cur_y = inner.y();

        for (idx, &layer) in SpacingLayer::ALL.iter().enumerate() {
            if cur_y.get().saturating_add(2) > inner.bottom().get() {
                break;
            }
            let val = self.spacing.get(layer);
            let max_val = layer.max_value();
            let is_sel = idx == self.cursor;

            let mut badge = StackStr::<16>::new();
            let _ = write!(badge, "{val} / {max_val}");

            let used = ChoiceRow::new((idx + 1) as u16, layer.label())
                .with_badge(badge.as_str())
                .with_badge_color(if val > 0 {
                    palette.accent
                } else {
                    palette.muted
                })
                .with_detail(layer.description())
                .with_selected(is_sel)
                .with_tty_mode(tty_mode)
                .render(buffer, inner.x(), cur_y, max_w, palette);
            cur_y = cur_y.saturating_add(used);

            if cur_y.get() < inner.bottom().get() && max_w > 8 {
                let slider =
                    SliderBar::new(i32::from(val), 0, i32::from(max_val)).with_tty_mode(tty_mode);
                let _ = slider.render(
                    buffer,
                    inner.x().saturating_add(6),
                    cur_y,
                    max_w.saturating_sub(6),
                    palette,
                );
                cur_y = cur_y.saturating_add(1);
            }
        }
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        let layer = SpacingLayer::ALL[self.cursor.min(3)];
        match key.code() {
            KeyCode::Esc | KeyCode::Enter => {
                let _ = ctx.backend.save_ui_prefs();
                ModalResult::CloseWithToast {
                    message: format!(
                        "Spacing saved (T-P:{} P-S:{} S-S:{} S-O:{})",
                        self.spacing.terminal_page_gap(),
                        self.spacing.page_section_gap(),
                        self.spacing.section_gap(),
                        self.spacing.section_option_gap(),
                    ),
                    is_error: false,
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = (self.cursor + 3) % 4;
                ModalResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1) % 4;
                ModalResult::Continue
            }
            KeyCode::Left | KeyCode::Char('h' | '-') => {
                let cur = self.spacing.get(layer);
                self.spacing.set(layer, cur.saturating_sub(1));
                *ctx.spacing = self.spacing;
                ModalResult::Continue
            }
            KeyCode::Right | KeyCode::Char('l' | '+' | '=') => {
                let cur = self.spacing.get(layer);
                self.spacing.set(layer, cur.saturating_add(1));
                *ctx.spacing = self.spacing;
                ModalResult::Continue
            }
            KeyCode::Char('r' | 'R') => {
                self.spacing.reset();
                *ctx.spacing = self.spacing;
                ModalResult::Continue
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                let digit = (ch as u16) - u16::from(b'0');
                self.spacing.set(layer, digit);
                *ctx.spacing = self.spacing;
                ModalResult::Continue
            }
            _ => ModalResult::Continue,
        }
    }
}

/// Constructs the universal `[19] Section Spacing Gaps` [`OptionSpec`] for any [`DashboardBackend`].
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, PrioritySegment};
/// use tui_rs::dashboard::DashboardBackend;
/// use tui_rs::universal::section_gap_option;
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let opt = section_gap_option::<App>(19);
/// assert_eq!(opt.number(), 19);
/// ```
#[must_use]
pub fn section_gap_option<B: DashboardBackend + 'static>(number: u16) -> OptionSpec<B> {
    OptionSpec::new(number, "Section Spacing Gaps", |ctx| {
        OptionOutcome::OpenModal(Box::new(SectionGapModal::new(*ctx.spacing)))
    })
    .with_badge(|_| ("4-Layer".to_string(), None))
}
