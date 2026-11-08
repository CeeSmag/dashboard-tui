//! Universal 6-palette color theme picker (`ColorThemeModal`, `color_theme_option`).
//!
//! Directly ports `ColorThemeModal` (`[20] Color Theme`) from
//! `ref/dashboard/addons/universal/theme.go`. Live-previews each [`ColorPalette`]
//! as the selection cursor moves and reverts to the initial palette on `Esc`.

use crate::core::buffer::Buffer;
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::dashboard::section::{OptionOutcome, OptionSpec};
use crate::widgets::choice_row::ChoiceRow;

/// Concise descriptions for the 6 built-in themes in [`ColorPalette::ALL_THEMES`].
const THEME_DESCRIPTIONS: [&str; 6] = [
    "High-contrast cyber blue & green dark palette",
    "Soothing pastel lavender & teal dark palette",
    "Vibrant purple, pink & cyan dark palette",
    "Arctic frost blue & snow dark palette",
    "High-contrast daylight terminal palette",
    "16-color ANSI palette for Linux console / serial TTY",
];

/// Interactive 6-palette color theme picker modal (`[20] Color Theme`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::ColorPalette;
/// use dashboard_tui::universal::ColorThemeModal;
///
/// let modal = ColorThemeModal::new(0, ColorPalette::DEFAULT);
/// assert_eq!(modal.cursor(), 0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ColorThemeModal {
    initial_index: usize,
    initial_palette: ColorPalette,
    cursor: usize,
}

impl ColorThemeModal {
    /// Creates a new [`ColorThemeModal`] remembering the current `initial_index` and `initial_palette`.
    #[must_use]
    #[inline]
    pub const fn new(initial_index: usize, initial_palette: ColorPalette) -> Self {
        let clamped = if initial_index < ColorPalette::ALL_THEMES.len() {
            initial_index
        } else {
            0
        };
        Self {
            initial_index: clamped,
            initial_palette,
            cursor: clamped,
        }
    }

    /// Returns the currently highlighted theme index (`0..6`).
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    fn apply_preview<B: DashboardBackend>(&self, ctx: &mut ModalContext<'_, B>) {
        let idx = self.cursor.min(ColorPalette::ALL_THEMES.len() - 1);
        *ctx.theme_index = idx;
        *ctx.palette = ColorPalette::ALL_THEMES[idx];
    }
}

impl<B: DashboardBackend> Modal<B> for ColorThemeModal {
    fn title(&self) -> &'static str {
        "Color Theme (Live Preview)"
    }

    fn subtitle(&self) -> &'static str {
        "↑↓/1-6 preview • Enter apply • Esc revert"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let w = (viewport.width().get() * 3 / 4).clamp(50, 72);
        (TerminalWidth::new(w), TerminalHeight::new(16))
    }

    #[expect(clippy::cast_possible_truncation, reason = "theme count is 6")]
    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool) {
        if inner.is_empty() {
            return;
        }
        let max_w = inner.width().get();
        let mut cur_y = inner.y();

        for (idx, theme) in ColorPalette::ALL_THEMES.iter().enumerate() {
            if cur_y.get() >= inner.bottom().get() {
                break;
            }
            let is_sel = idx == self.cursor;
            let badge = if idx == self.initial_index {
                "● CURRENT"
            } else if is_sel {
                "◉ PREVIEW"
            } else {
                ""
            };
            let desc = THEME_DESCRIPTIONS.get(idx).copied().unwrap_or("");

            let row_w = max_w.saturating_sub(10);
            let used = ChoiceRow::new((idx + 1) as u16, theme.name)
                .with_badge(badge)
                .with_badge_color(theme.accent)
                .with_detail(desc)
                .with_selected(is_sel)
                .with_tty_mode(tty_mode)
                .render(buffer, inner.x(), cur_y, row_w, palette);

            // Render 4 color swatch blocks `[■■■■]` on the right edge of the choice row
            if max_w >= 20 {
                let swatch_x = inner.right().saturating_sub(8);
                let glyph = if tty_mode { "#" } else { "■" };
                let colors = [theme.accent, theme.ok, theme.warn, theme.err];
                let _ = buffer.set_cell(swatch_x, cur_y, "[", Style::new().with_fg(palette.dim));
                for (ci, &c) in colors.iter().enumerate() {
                    let _ = buffer.set_cell(
                        swatch_x.saturating_add((ci + 1) as u16),
                        cur_y,
                        glyph,
                        Style::new().with_fg(c),
                    );
                }
                let _ = buffer.set_cell(
                    swatch_x.saturating_add(5),
                    cur_y,
                    "]",
                    Style::new().with_fg(palette.dim),
                );
            }

            cur_y = cur_y.saturating_add(used);
        }
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        let count = ColorPalette::ALL_THEMES.len();
        match key.code() {
            KeyCode::Esc | KeyCode::Char('q') => {
                *ctx.theme_index = self.initial_index;
                *ctx.palette = self.initial_palette;
                ModalResult::Close
            }
            KeyCode::Enter => {
                self.apply_preview(ctx);
                let _ = ctx.backend.save_ui_prefs();
                let name = ColorPalette::ALL_THEMES[self.cursor].name;
                ModalResult::CloseWithToast {
                    message: format!("Color theme set to {name}"),
                    is_error: false,
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = (self.cursor + count - 1) % count;
                self.apply_preview(ctx);
                ModalResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cursor = (self.cursor + 1) % count;
                self.apply_preview(ctx);
                ModalResult::Continue
            }
            KeyCode::Char(ch @ '1'..='6') => {
                self.cursor = (ch as usize) - (b'1' as usize);
                self.apply_preview(ctx);
                ModalResult::Continue
            }
            _ => ModalResult::Continue,
        }
    }
}

/// Constructs the universal `[20] Color Theme` [`OptionSpec`] for any [`DashboardBackend`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::DashboardBackend;
/// use dashboard_tui::universal::color_theme_option;
///
/// struct App;
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "VaultDrive" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let opt = color_theme_option::<App>(20);
/// assert_eq!(opt.number(), 20);
/// ```
#[must_use]
pub fn color_theme_option<B: DashboardBackend + 'static>(number: u16) -> OptionSpec<B> {
    OptionSpec::new(number, "Color Theme", |ctx| {
        OptionOutcome::OpenModal(Box::new(ColorThemeModal::new(
            *ctx.theme_index,
            *ctx.palette,
        )))
    })
    .with_badge(|_| ("6 Palettes".to_string(), None))
}
