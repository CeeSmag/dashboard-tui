//! Horizontal tab strip (`Tabs`) with a highlighted selection.
//!
//! When the titles do not fit, the strip scrolls so the selected tab is always visible.

use crate::core::buffer::Buffer;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::text::Line;
use crate::core::widget::Widget;

/// A row of tab titles with one selected.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Rect, Style, Widget};
/// use dashboard_tui::widgets::Tabs;
///
/// let area = Rect::from_xywh(0, 0, 25, 1);
/// let mut buffer = Buffer::empty(area);
/// Tabs::new(["Monitor", "Setup", "Theme"])
///     .select(1)
///     .highlight_style(Style::new().bold())
///     .render(area, &mut buffer);
/// assert_eq!(buffer.to_visual_lines(), vec![" Monitor │ Setup │ Theme "]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tabs<'a> {
    titles: Vec<Line<'a>>,
    selected: usize,
    style: Style,
    highlight_style: Style,
    divider: &'a str,
    padding_left: &'a str,
    padding_right: &'a str,
}

impl<'a> Tabs<'a> {
    /// Creates tabs from titles (`&str`, `String`, [`Line`], …) with the first selected.
    #[must_use]
    pub fn new<T>(titles: impl IntoIterator<Item = T>) -> Self
    where
        T: Into<Line<'a>>,
    {
        Self {
            titles: titles.into_iter().map(Into::into).collect(),
            selected: 0,
            style: Style::DEFAULT,
            highlight_style: Style::DEFAULT.reversed(),
            divider: "│",
            padding_left: " ",
            padding_right: " ",
        }
    }

    /// Selects the tab at `index` (clamped when rendering).
    #[must_use]
    pub const fn select(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Sets the style of unselected tabs and the gaps between them.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the style of the selected tab (default: reversed).
    #[must_use]
    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Sets the divider drawn between tabs (default `"│"`; use `"|"` on a Linux console).
    #[must_use]
    pub const fn divider(mut self, divider: &'a str) -> Self {
        self.divider = divider;
        self
    }

    /// Sets the padding drawn on each side of every title (default one space).
    #[must_use]
    pub const fn padding(mut self, left: &'a str, right: &'a str) -> Self {
        self.padding_left = left;
        self.padding_right = right;
        self
    }

    fn tab_width(&self, index: usize) -> usize {
        self.titles.get(index).map_or(0, |title| {
            usize::from(title.width().as_u16())
                + usize::from(crate::core::grapheme::display_width(self.padding_left).as_u16())
                + usize::from(crate::core::grapheme::display_width(self.padding_right).as_u16())
        })
    }
}

impl Widget for Tabs<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() || self.titles.is_empty() {
            return;
        }
        buffer.set_style(area, self.style);
        let selected = self.selected.min(self.titles.len() - 1);
        let divider_width =
            usize::from(crate::core::grapheme::display_width(self.divider).as_u16());
        let available = usize::from(area.width().get());

        // Scroll: choose the first tab so that tabs first..=selected fit in the width.
        let mut first = selected;
        let mut used = self.tab_width(selected);
        while first > 0 {
            let candidate = used
                .saturating_add(divider_width)
                .saturating_add(self.tab_width(first - 1));
            if candidate > available {
                break;
            }
            used = candidate;
            first -= 1;
        }

        let right_edge = area.right();
        let row = area.y();
        let mut column = area.x();
        for index in first..self.titles.len() {
            if column >= right_edge {
                break;
            }
            let is_selected = index == selected;
            let tab_style = if is_selected {
                self.highlight_style
            } else {
                self.style
            };
            column = buffer.set_str_clipped(column, row, self.padding_left, right_edge, tab_style);
            if let Some(title) = self.titles.get(index) {
                let width = right_edge.get().saturating_sub(column.get());
                let start = column;
                column = buffer.set_line(
                    column,
                    row,
                    title,
                    crate::core::coordinates::TerminalWidth::new(width.min(title.width().as_u16())),
                );
                if is_selected {
                    buffer.set_style(
                        Rect::new(
                            start,
                            row,
                            crate::core::coordinates::TerminalWidth::new(
                                column.get().saturating_sub(start.get()),
                            ),
                            crate::core::coordinates::TerminalHeight::new(1),
                        ),
                        self.highlight_style,
                    );
                }
            }
            column = buffer.set_str_clipped(column, row, self.padding_right, right_edge, tab_style);
            if index + 1 < self.titles.len() {
                column = buffer.set_str_clipped(column, row, self.divider, right_edge, self.style);
            }
        }
    }
}
