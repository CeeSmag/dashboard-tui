//! Uniform widget contracts: [`Widget`] and [`StatefulWidget`].
//!
//! A widget draws itself into a [`Buffer`] region. Widgets are plain values (builders) that
//! are consumed when rendered; keep them cheap to construct, or implement the trait on a
//! reference type for reusable widgets. Stateful widgets additionally borrow a state
//! object (selection, scroll offset) that survives between frames.

use crate::core::buffer::Buffer;
use crate::core::rect::Rect;

/// A value that can draw itself into a rectangular region of a [`Buffer`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Rect, Style, Widget};
///
/// struct Banner(&'static str);
/// impl Widget for Banner {
///     fn render(self, area: Rect, buffer: &mut Buffer) {
///         let _ = buffer.set_string(area.x(), area.y(), self.0, Style::DEFAULT, area.width().get());
///     }
/// }
///
/// let area = Rect::from_xywh(0, 0, 8, 1);
/// let mut buffer = Buffer::empty(area);
/// buffer.render_widget(Banner("hello"), area);
/// assert_eq!(buffer.to_visual_lines(), vec!["hello   "]);
/// ```
pub trait Widget {
    /// Draws the widget into `area` of `buffer`. Must not panic for any `area`.
    fn render(self, area: Rect, buffer: &mut Buffer)
    where
        Self: Sized;
}

/// A widget that also reads and updates a persistent `State` while rendering.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Rect, StatefulWidget, Style};
///
/// struct Counter;
/// impl StatefulWidget for Counter {
///     type State = u32;
///     fn render(self, area: Rect, buffer: &mut Buffer, state: &mut u32) {
///         *state += 1;
///         let text = if *state == 1 { "first" } else { "again" };
///         let _ = buffer.set_string(area.x(), area.y(), text, Style::DEFAULT, area.width().get());
///     }
/// }
///
/// let area = Rect::from_xywh(0, 0, 5, 1);
/// let mut buffer = Buffer::empty(area);
/// let mut calls = 0;
/// buffer.render_stateful_widget(Counter, area, &mut calls);
/// buffer.render_stateful_widget(Counter, area, &mut calls);
/// assert_eq!(calls, 2);
/// assert_eq!(buffer.to_visual_lines(), vec!["again"]);
/// ```
pub trait StatefulWidget {
    /// The state type the widget mutates while rendering.
    type State: ?Sized;

    /// Draws the widget into `area` of `buffer`, reading and updating `state`.
    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized;
}

impl Buffer {
    /// Renders `widget` into `area` of this buffer.
    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self);
    }

    /// Renders a [`StatefulWidget`] into `area` of this buffer using `state`.
    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) {
        widget.render(area, self, state);
    }
}
