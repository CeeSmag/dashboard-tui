//! Modal overlay trait (`Modal`), execution context (`ModalContext`), and
//! dimmed backdrop buffer splicing (`splice_modal_overlay`).
//!
//! Directly ports `Modal` and `spliceModal` from `ref/dashboard/de_dashboard.go`.
//! Instead of blanking the screen when a modal opens, the underlying page
//! buffer is dimmed in-place and the centered `TabPanel` modal frame is drawn
//! directly over the dimmed backdrop.

use crate::core::buffer::Buffer;
use crate::core::color::Color;
use crate::core::palette::ColorPalette;
use crate::core::palette::TerminalBackground;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyBindings, KeyPress};
use crate::dashboard::spacing::SpacingConfig;
use crate::widgets::panel::TabPanel;

/// Mutable context passed to [`Modal::on_key`] so modals can inspect or
/// live-preview changes to the backend, 4-layer spacing, or color palette.
#[derive(Debug)]
pub struct ModalContext<'a, B: DashboardBackend> {
    /// Mutable reference to the host domain backend.
    pub backend: &'a mut B,
    /// Mutable reference to the engine's 4-layer spacing configuration.
    pub spacing: &'a mut SpacingConfig,
    /// Mutable reference to the engine's active color palette.
    pub palette: &'a mut ColorPalette,
    /// Mutable reference to the active preset theme index.
    pub theme_index: &'a mut usize,
    /// Mutable reference to the engine's terminal background mode.
    pub terminal_background: &'a mut TerminalBackground,
    /// Whether Linux TTY safe glyph mode is active.
    pub tty_mode: bool,
    /// The engine's active global shortkeys, so custom modals and sub-views can honour
    /// the developer's `up`/`down`/`activate`/`cancel` choices.
    pub keys: &'a KeyBindings,
}

/// Outcome returned by [`Modal::on_key`].
///
/// # Examples
///
/// ```
/// use tui_rs::dashboard::ModalResult;
///
/// let res = ModalResult::Close;
/// assert!(matches!(res, ModalResult::Close));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModalResult {
    /// Keep the modal open. The global cancel key (`q` by default) still closes it.
    Continue,
    /// The key was consumed as input (for example a typed character): keep the modal open and do
    /// **not** apply the global cancel key, so a text field can receive the letter `q`.
    Consumed,
    /// Close the modal without a toast notification.
    Close,
    /// Close the modal and display a temporary toast message in the footer bar.
    CloseWithToast {
        /// Toast notification message.
        message: String,
        /// `true` if the toast represents an error/warning, `false` for success/info.
        is_error: bool,
    },
    /// Close the modal and request clean termination of the dashboard loop
    /// (see [`EngineAction::Quit`](crate::dashboard::EngineAction::Quit)).
    Quit,
}

/// Trait implemented by interactive modal overlays (`EditModal`, `ConfirmModal`,
/// `TypedModal`, `NoticeModal`, `SectionGapModal`, `ColorThemeModal`, or custom add-on modals).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
/// use tui_rs::dashboard::{DashboardBackend, KeyCode, KeyPress, Modal, ModalContext, ModalResult};
///
/// struct SimpleNotice;
///
/// impl<B: DashboardBackend> Modal<B> for SimpleNotice {
///     fn title(&self) -> &str {
///         "Notice"
///     }
///     fn desired_size(&self, _viewport: Rect) -> (TerminalWidth, TerminalHeight) {
///         (TerminalWidth::new(40), TerminalHeight::new(8))
///     }
///     fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, _tty_mode: bool) {
///         let _ = (buffer, inner, palette);
///     }
///     fn on_key(&mut self, key: KeyPress, _ctx: &mut ModalContext<'_, B>) -> ModalResult {
///         if key.code() == KeyCode::Esc || key.code() == KeyCode::Enter {
///             ModalResult::Close
///         } else {
///             ModalResult::Continue
///         }
///     }
/// }
/// ```
pub trait Modal<B: DashboardBackend> {
    /// Title rendered in the modal's top `TabPanel` notch.
    fn title(&self) -> &str;

    /// Subtitle / key hint rendered in the modal's bottom `TabPanel` notch.
    fn subtitle(&self) -> &'static str {
        "Enter confirm • Esc cancel"
    }

    /// Desired outer `(width, height)` of the modal `TabPanel` box given `viewport`.
    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight);

    /// Renders the modal's inner body into `inner` inside `buffer`.
    fn render(&self, buffer: &mut Buffer, inner: Rect, palette: &ColorPalette, tty_mode: bool);

    /// Renders the modal body with read access to the host `backend`.
    ///
    /// The engine calls this one. Override it when the body shows live backend values (as
    /// [`SettingsModal`](crate::universal::SettingsModal) does); the default forwards to
    /// [`Modal::render`].
    fn render_with_backend(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        let _ = backend;
        self.render(buffer, inner, palette, tty_mode);
    }

    /// Processes a keyboard press while the modal is active.
    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult;
}

/// Dims every cell in `area` inside `buffer` by replacing foreground colors with
/// `palette.dim` and clearing background highlights, creating a visual backdrop
/// for a modal overlay.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, Rect, Style, TerminalHeight, TerminalWidth};
/// use tui_rs::dashboard::dim_buffer_backdrop;
///
/// let area = Rect::from_size(TerminalWidth::new(10), TerminalHeight::new(2));
/// let mut buf = Buffer::empty(area);
/// buf.set_string(area.x(), area.y(), "Active", Style::default(), 10);
/// dim_buffer_backdrop(&mut buf, area, &ColorPalette::DEFAULT);
/// ```
pub fn dim_buffer_backdrop(buffer: &mut Buffer, area: Rect, palette: &ColorPalette) {
    let clipped = area.intersection(buffer.area());
    let dim_style = Style::new().with_fg(palette.dim).with_bg(Color::Reset);
    for row_offset in 0..clipped.height().get() {
        let ry = clipped.y().saturating_add(row_offset);
        for col_offset in 0..clipped.width().get() {
            let cx = clipped.x().saturating_add(col_offset);
            if let Some(cell) = buffer.get_mut(cx, ry) {
                cell.set_style(dim_style);
            }
        }
    }
}

/// Dims `backdrop_area` in `buffer`, clears the centered modal rectangle, renders
/// the modal's outer [`TabPanel`] border, and invokes [`Modal::render`] on the
/// inner area.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Color, ColorPalette, PrioritySegment, Rect, TerminalHeight, TerminalWidth};
/// use tui_rs::dashboard::{
///     splice_modal_overlay, DashboardBackend, KeyPress, Modal, ModalContext, ModalResult,
/// };
///
/// struct DummyBackend;
/// impl DashboardBackend for DummyBackend {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// struct DummyModal;
/// impl Modal<DummyBackend> for DummyModal {
///     fn title(&self) -> &str { "Confirm" }
///     fn desired_size(&self, _: Rect) -> (TerminalWidth, TerminalHeight) {
///         (TerminalWidth::new(24), TerminalHeight::new(6))
///     }
///     fn render(&self, _: &mut Buffer, _: Rect, _: &ColorPalette, _: bool) {}
///     fn on_key(&mut self, _: KeyPress, _: &mut ModalContext<'_, DummyBackend>) -> ModalResult {
///         ModalResult::Close
///     }
/// }
///
/// let area = Rect::from_size(TerminalWidth::new(60), TerminalHeight::new(20));
/// let mut buf = Buffer::empty(area);
/// splice_modal_overlay(&DummyModal, &mut buf, area, &ColorPalette::DEFAULT, false);
/// ```
pub fn splice_modal_overlay<B: DashboardBackend>(
    modal: &dyn Modal<B>,
    buffer: &mut Buffer,
    backdrop_area: Rect,
    palette: &ColorPalette,
    tty_mode: bool,
) {
    splice_modal_frame(
        modal,
        buffer,
        backdrop_area,
        palette,
        tty_mode,
        |modal, buffer, inner| modal.render(buffer, inner, palette, tty_mode),
    );
}

/// Same as [`splice_modal_overlay`], but draws the body through
/// [`Modal::render_with_backend`] so modals can show live backend values.
pub fn splice_modal_overlay_with_backend<B: DashboardBackend>(
    modal: &dyn Modal<B>,
    backend: &B,
    buffer: &mut Buffer,
    backdrop_area: Rect,
    palette: &ColorPalette,
    tty_mode: bool,
) {
    splice_modal_frame(
        modal,
        buffer,
        backdrop_area,
        palette,
        tty_mode,
        |modal, buffer, inner| {
            modal.render_with_backend(backend, buffer, inner, palette, tty_mode);
        },
    );
}

fn splice_modal_frame<B: DashboardBackend>(
    modal: &dyn Modal<B>,
    buffer: &mut Buffer,
    backdrop_area: Rect,
    palette: &ColorPalette,
    tty_mode: bool,
    draw_body: impl FnOnce(&dyn Modal<B>, &mut Buffer, Rect),
) {
    if backdrop_area.is_empty() {
        return;
    }
    dim_buffer_backdrop(buffer, backdrop_area, palette);

    let (req_w, req_h) = modal.desired_size(backdrop_area);
    let max_w = backdrop_area.width().get().saturating_sub(2).max(16);
    let max_h = backdrop_area.height().get().saturating_sub(2).max(5);
    let box_w = TerminalWidth::new(req_w.get().clamp(16.min(max_w), max_w));
    let box_h = TerminalHeight::new(req_h.get().clamp(5.min(max_h), max_h));

    let modal_rect = backdrop_area.centered(box_w, box_h);

    // Clear the modal rectangle so underlying dimmed characters do not bleed through
    buffer.fill_rect(modal_rect, ' ', Style::new().with_fg(palette.text));

    let panel = TabPanel::new(modal.title())
        .with_subtitle(modal.subtitle())
        .with_focused(true)
        .with_tty_mode(tty_mode);
    let inner = panel.render(buffer, modal_rect, palette);
    if !inner.is_empty() {
        draw_body(modal, buffer, inner);
    }
}
