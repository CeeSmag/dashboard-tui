//! Blank-out widget (`Clear`) for wiping an area before drawing a popup on top of it.

use crate::core::buffer::Buffer;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::widget::Widget;

/// Resets every cell in its area to a blank space (dropping glyphs, styles and hyperlinks).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style, Widget};
/// use tui_rs::widgets::Clear;
///
/// let full = Rect::from_xywh(0, 0, 5, 1);
/// let mut buffer = Buffer::empty(full);
/// let _ = buffer.set_string(ColumnIndex(0), RowIndex(0), "hello", Style::DEFAULT, 5);
/// Clear.render(Rect::from_xywh(1, 0, 3, 1), &mut buffer);
/// assert_eq!(buffer.to_visual_lines(), vec!["h   o"]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Clear;

impl Widget for Clear {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        buffer.clear_area(area, Style::DEFAULT);
    }
}
