//! Scrollable, selectable list (`List`, `ListItem`, `ListState`).
//!
//! The list keeps its selection and scroll offset in a [`ListState`] owned by the
//! application, so the highlighted row and scroll position survive between frames.
//! Rendering follows the selection (with optional scroll padding) and never allocates.

use crate::core::buffer::Buffer;
use crate::core::grapheme::display_width;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::core::text::Line;
use crate::core::widget::StatefulWidget;

/// Selection and scroll position of a [`List`] (or [`Table`](crate::widgets::Table)).
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::ListState;
///
/// let mut state = ListState::default();
/// assert_eq!(state.selected(), None);
/// state.select_next();
/// assert_eq!(state.selected(), Some(0));
/// state.select_next();
/// state.select_previous();
/// assert_eq!(state.selected(), Some(0));
/// state.select_last();
/// // `select_last` is clamped to the real length the next time the widget renders.
/// assert!(state.selected().is_some());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ListState {
    selected: Option<usize>,
    offset: usize,
}

impl ListState {
    /// Creates a state with nothing selected, scrolled to the top.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected: None,
            offset: 0,
        }
    }

    /// Returns the state with `index` selected.
    #[must_use]
    pub const fn with_selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Returns the selected row index, if any.
    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Selects `index` (or clears the selection and scrolls to the top with `None`).
    pub const fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }

    /// Returns the index of the first visible row.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Sets the index of the first visible row (clamped when the widget renders).
    pub const fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Clamps the selection to the last of `len` rows (no-op when `len` is zero).
    pub(crate) const fn set_selected_clamped(&mut self, len: usize) {
        if let Some(index) = self.selected {
            if len > 0 && index >= len {
                self.selected = Some(len - 1);
            }
        }
    }

    /// Selects the next row (the first one when nothing is selected). Clamped on render.
    pub const fn select_next(&mut self) {
        self.selected = Some(match self.selected {
            Some(index) => index.saturating_add(1),
            None => 0,
        });
    }

    /// Selects the previous row (the first one when nothing is selected).
    pub const fn select_previous(&mut self) {
        self.selected = Some(match self.selected {
            Some(index) => index.saturating_sub(1),
            None => 0,
        });
    }

    /// Selects the first row.
    pub const fn select_first(&mut self) {
        self.selected = Some(0);
    }

    /// Selects the last row (clamped to the real length on render).
    pub const fn select_last(&mut self) {
        self.selected = Some(usize::MAX);
    }

    /// Moves the selection down by `amount` rows.
    pub const fn scroll_down_by(&mut self, amount: usize) {
        self.selected = Some(match self.selected {
            Some(index) => index.saturating_add(amount),
            None => 0,
        });
    }

    /// Moves the selection up by `amount` rows.
    pub const fn scroll_up_by(&mut self, amount: usize) {
        self.selected = Some(match self.selected {
            Some(index) => index.saturating_sub(amount),
            None => 0,
        });
    }
}

/// Computes the new scroll offset that keeps `selected` visible with `padding` rows of context.
///
/// `len` is the total row count, `height` the number of visible rows.
#[must_use]
pub(crate) fn follow_selection(
    offset: usize,
    selected: Option<usize>,
    len: usize,
    height: usize,
    padding: usize,
) -> usize {
    if height == 0 || len == 0 {
        return 0;
    }
    let furthest_offset = len.saturating_sub(height);
    let mut new_offset = offset.min(furthest_offset);
    if let Some(index) = selected {
        let padding = padding.min(height.saturating_sub(1) / 2);
        if index < new_offset.saturating_add(padding) {
            new_offset = index.saturating_sub(padding);
        } else if index.saturating_add(padding) >= new_offset.saturating_add(height) {
            new_offset = index
                .saturating_add(padding)
                .saturating_add(1)
                .saturating_sub(height);
        }
    }
    new_offset.min(furthest_offset)
}

/// One row of a [`List`].
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Color, Style};
/// use tui_rs::widgets::ListItem;
///
/// let item = ListItem::new("Mount vault").style(Style::new().fg(Color::Green));
/// assert_eq!(item.line().width().get(), 11);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListItem<'a> {
    line: Line<'a>,
}

impl<'a> ListItem<'a> {
    /// Creates an item from anything convertible to a [`Line`] (`&str`, `String`, `Span`, …).
    #[must_use]
    pub fn new(content: impl Into<Line<'a>>) -> Self {
        Self {
            line: content.into(),
        }
    }

    /// Sets the item's base style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.line = self.line.style(style);
        self
    }

    /// Returns the item's line.
    #[must_use]
    pub const fn line(&self) -> &Line<'a> {
        &self.line
    }
}

impl<'a, T: Into<Line<'a>>> From<T> for ListItem<'a> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// A scrollable list of single-line items with a selection highlight.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Rect, Style, StatefulWidget};
/// use tui_rs::widgets::{List, ListState};
///
/// let area = Rect::from_xywh(0, 0, 12, 3);
/// let mut buffer = Buffer::empty(area);
/// let mut state = ListState::default().with_selected(Some(3));
/// let list = List::new(["one", "two", "three", "four", "five"])
///     .highlight_symbol("> ")
///     .highlight_style(Style::new().bold());
/// buffer.render_stateful_widget(list, area, &mut state);
///
/// // The list scrolled so the selected row is visible.
/// assert_eq!(state.offset(), 1);
/// assert!(buffer.to_visual_lines()[2].starts_with("> four"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct List<'a> {
    items: Vec<ListItem<'a>>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: &'a str,
    scroll_padding: usize,
}

impl<'a> List<'a> {
    /// Creates a list from items (`&str`, `String`, [`Line`], or [`ListItem`] values).
    #[must_use]
    pub fn new<T>(items: impl IntoIterator<Item = T>) -> Self
    where
        T: Into<ListItem<'a>>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            style: Style::DEFAULT,
            highlight_style: Style::DEFAULT.reversed(),
            highlight_symbol: "",
            scroll_padding: 0,
        }
    }

    /// Sets the base style applied behind every row.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the style of the selected row (default: reversed).
    #[must_use]
    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Sets the marker drawn before the selected row (for example `"> "`). Space for it is
    /// reserved on every row whenever something is selected, so rows never shift.
    #[must_use]
    pub const fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = symbol;
        self
    }

    /// Keeps `rows` rows of context between the selection and the viewport edge.
    #[must_use]
    pub const fn scroll_padding(mut self, rows: usize) -> Self {
        self.scroll_padding = rows;
        self
    }

    /// Returns the number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the list has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl StatefulWidget for List<'_> {
    type State = ListState;

    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut ListState) {
        let len = self.items.len();
        let height = usize::from(area.height().get());
        state.set_selected_clamped(len);
        state.offset = follow_selection(
            state.offset,
            state.selected,
            len,
            height,
            self.scroll_padding,
        );
        if area.is_empty() || len == 0 {
            state.offset = state.offset.min(len.saturating_sub(1));
            return;
        }
        buffer.set_style(area, self.style);

        let symbol_width = if state.selected.is_some() {
            display_width(self.highlight_symbol)
                .as_u16()
                .min(area.width().get())
        } else {
            0
        };
        let text_width = area.width().get().saturating_sub(symbol_width);

        for (row_offset, (index, item)) in self
            .items
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(height)
            .enumerate()
        {
            let row_index = u16::try_from(row_offset).unwrap_or(u16::MAX);
            let row = area.y().saturating_add(row_index);
            let is_selected = state.selected == Some(index);
            if is_selected {
                buffer.set_style(
                    Rect::new(area.x(), row, area.width(), TerminalHeight::new(1)),
                    self.highlight_style,
                );
                if symbol_width > 0 {
                    let _ = buffer.set_string(
                        area.x(),
                        row,
                        self.highlight_symbol,
                        self.highlight_style,
                        symbol_width,
                    );
                }
            }
            let _ = buffer.set_line(
                area.x().saturating_add(symbol_width),
                row,
                item.line(),
                TerminalWidth::new(text_width),
            );
        }
    }
}
