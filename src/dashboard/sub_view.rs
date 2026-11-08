//! Full-area drill-down sub-view stack (`SubView`, `SubViewContext`, `SubViewResult`).
//!
//! Directly ports `SubView`, `PushView`, and `PopView` from `ref/dashboard/de_dashboard.go`.
//! While a [`SubView`] is on top of the stack, it owns the entire page body area
//! (between the top header bar and bottom footer bar) and receives all keyboard
//! input until it returns [`SubViewResult::Pop`].

use crate::core::buffer::Buffer;
use crate::core::palette::ColorPalette;
use crate::core::priority_strip::KeyHint;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::KeyPress;
use crate::dashboard::modal::ModalContext;
use crate::dashboard::section::OptionOutcome;

/// Outcome returned by [`SubView::on_key`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::dashboard::SubViewResult;
///
/// assert_eq!(SubViewResult::Pop, SubViewResult::Pop);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubViewResult {
    /// Keep the sub-view active on the stack.
    Continue,
    /// Pop this sub-view off the stack and return to the underlying view or page.
    Pop,
    /// Pop this sub-view and display a toast notification in the footer bar.
    PopWithToast {
        /// Toast message text.
        message: String,
        /// `true` if error/warning, `false` if success/info.
        is_error: bool,
    },
}

/// What a [`SubView`] asks the engine to do after a key press (see
/// [`SubView::on_key_with_context`]).
pub enum SubViewAction<B: DashboardBackend> {
    /// A plain [`SubViewResult`]: stay, pop, or pop with a toast.
    Result(SubViewResult),
    /// Stay open and apply an [`OptionOutcome`] (toast, modal, deeper sub-view, quit).
    Outcome(OptionOutcome<B>),
}

impl<B: DashboardBackend> core::fmt::Debug for SubViewAction<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Result(result) => f.debug_tuple("Result").field(result).finish(),
            Self::Outcome(outcome) => f.debug_tuple("Outcome").field(outcome).finish(),
        }
    }
}

/// Trait implemented by full-area drill-down sub-views pushed onto the
/// [`DashboardEngine`](crate::dashboard::DashboardEngine) view stack.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, ColorPalette, KeyHint, Rect};
/// use dashboard_tui::dashboard::{DashboardBackend, KeyCode, KeyPress, SubView, SubViewResult};
///
/// struct LogDetailSubView;
///
/// impl<B: DashboardBackend> SubView<B> for LogDetailSubView {
///     fn title(&self) -> &str {
///         "Event Stream Inspector"
///     }
///     fn render(&self, _backend: &B, _buffer: &mut Buffer, _area: Rect, _palette: &ColorPalette, _tty_mode: bool) {}
///     fn on_key(&mut self, _backend: &mut B, key: KeyPress) -> SubViewResult {
///         if key.code() == KeyCode::Esc {
///             SubViewResult::Pop
///         } else {
///             SubViewResult::Continue
///         }
///     }
/// }
/// ```
pub trait SubView<B: DashboardBackend> {
    /// Title of the sub-view.
    fn title(&self) -> &str;

    /// Renders the sub-view into `area` inside `buffer`.
    fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    );

    /// Handles a key press while this sub-view is on top of the stack.
    fn on_key(&mut self, backend: &mut B, key: KeyPress) -> SubViewResult;

    /// Context-aware variant of [`SubView::on_key`]. The engine calls this one. Override it
    /// when the sub-view needs the active [`KeyBindings`](crate::dashboard::KeyBindings),
    /// spacing or palette, or wants to open a modal / push a deeper sub-view through an
    /// [`OptionOutcome`]. The default simply forwards to [`SubView::on_key`].
    fn on_key_with_context(
        &mut self,
        key: KeyPress,
        ctx: &mut ModalContext<'_, B>,
    ) -> SubViewAction<B> {
        SubViewAction::Result(self.on_key(ctx.backend, key))
    }

    /// Returns context-sensitive key hints displayed in the bottom footer bar
    /// while this sub-view is active.
    fn footer_hints(&self) -> Vec<KeyHint<'_>> {
        vec![
            KeyHint::new("↑↓", "scroll", Style::DEFAULT, 80),
            KeyHint::new("Esc", "back", Style::DEFAULT, 100),
        ]
    }
}
