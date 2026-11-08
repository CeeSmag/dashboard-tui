//! Multi-line styled text block (`Paragraph`) with wrapping, alignment and scrolling.
//!
//! Wrapping works directly over styled [`Span`]s with a byte-offset cursor, so rendering
//! needs no allocation. Words that straddle two differently styled spans are treated as
//! separate chunks when deciding where to break.

use unicode_segmentation::UnicodeSegmentation;

use crate::core::buffer::Buffer;
use crate::core::coordinates::RowIndex;
use crate::core::grapheme::grapheme_width;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::text::{Line, Span, Text, TextAlignment};
use crate::core::widget::Widget;

/// How a [`Paragraph`] handles lines wider than its area.
///
/// # Examples
///
/// ```
/// use dashboard_tui::widgets::WrapMode;
/// assert_eq!(WrapMode::default(), WrapMode::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WrapMode {
    /// Clip long lines (horizontal scrolling is available).
    #[default]
    None,
    /// Break anywhere a character no longer fits.
    Char,
    /// Break at whitespace when possible, falling back to a character break for long words.
    Word,
}

/// Position inside a [`Line`]: span index plus byte offset within that span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Cursor {
    span: usize,
    byte: usize,
}

/// Returns the next grapheme at `cursor` (advancing it), or `None` at the end of the line.
fn next_grapheme<'t>(
    spans: &'t [Span<'_>],
    cursor: &mut Cursor,
) -> Option<(&'t str, &'t Span<'t>)> {
    loop {
        let span = spans.get(cursor.span)?;
        // The cursor only ever advances by whole grapheme lengths, so `byte` is a boundary.
        let rest = span.as_str().get(cursor.byte..).unwrap_or("");
        if let Some(grapheme) = rest.graphemes(true).next() {
            cursor.byte = cursor.byte.saturating_add(grapheme.len());
            return Some((grapheme, span));
        }
        cursor.span = cursor.span.saturating_add(1);
        cursor.byte = 0;
    }
}

/// Finds where the visual row starting at `start` ends.
///
/// Returns `(end, used_width)`. Always makes progress when `width > 0` and text remains.
fn measure_row(spans: &[Span<'_>], start: Cursor, width: u16, mode: WrapMode) -> (Cursor, u16) {
    let mut cursor = start;
    let mut used: u16 = 0;
    let mut last_break: Option<(Cursor, u16)> = None;

    loop {
        let before = cursor;
        let Some((grapheme, _)) = next_grapheme(spans, &mut cursor) else {
            return (cursor, used);
        };
        let grapheme_columns = grapheme_width(grapheme);
        if grapheme_columns == 0 {
            continue;
        }
        if used.saturating_add(grapheme_columns) > width {
            if mode == WrapMode::Word {
                if let Some(found) = last_break {
                    if found.0 != start {
                        return found;
                    }
                }
            }
            if used == 0 {
                // Nothing fits (zero width, or a wide grapheme in a 1-column area): force progress.
                return (cursor, used);
            }
            return (before, used);
        }
        used = used.saturating_add(grapheme_columns);
        if grapheme.chars().all(char::is_whitespace) {
            last_break = Some((cursor, used));
        }
    }
}

/// Returns the cursor after skipping whitespace graphemes from `cursor`.
fn skip_whitespace(spans: &[Span<'_>], cursor: Cursor) -> Cursor {
    let mut position = cursor;
    loop {
        let mut probe = position;
        match next_grapheme(spans, &mut probe) {
            Some((grapheme, _)) if grapheme.chars().all(char::is_whitespace) => position = probe,
            _ => return position,
        }
    }
}

/// A block of styled text with optional wrapping, alignment and scrolling.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Rect, Style, TextAlignment, Widget};
/// use dashboard_tui::widgets::{Paragraph, WrapMode};
///
/// let area = Rect::from_xywh(0, 0, 10, 3);
/// let mut buffer = Buffer::empty(area);
/// Paragraph::new("the quick brown fox")
///     .wrap(WrapMode::Word)
///     .style(Style::new().dim())
///     .render(area, &mut buffer);
/// assert_eq!(buffer.to_visual_lines(), vec!["the quick ", "brown fox ", "          "]);
///
/// let centered = Paragraph::new("hi").alignment(TextAlignment::Center);
/// let mut small = Buffer::empty(Rect::from_xywh(0, 0, 6, 1));
/// centered.render(Rect::from_xywh(0, 0, 6, 1), &mut small);
/// assert_eq!(small.to_visual_lines(), vec!["  hi  "]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph<'a> {
    text: Text<'a>,
    style: Style,
    alignment: TextAlignment,
    wrap: WrapMode,
    trim: bool,
    scroll_rows: u16,
    scroll_columns: u16,
}

impl<'a> Paragraph<'a> {
    /// Creates a paragraph from anything convertible to [`Text`] (`&str`, `String`, [`Line`], …).
    #[must_use]
    pub fn new(text: impl Into<Text<'a>>) -> Self {
        Self {
            text: text.into(),
            style: Style::DEFAULT,
            alignment: TextAlignment::Left,
            wrap: WrapMode::None,
            trim: true,
            scroll_rows: 0,
            scroll_columns: 0,
        }
    }

    /// Sets the base style behind the whole paragraph.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the default alignment (a [`Line`] with its own non-left alignment keeps it).
    #[must_use]
    pub const fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the wrapping mode (default: clip).
    #[must_use]
    pub const fn wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    /// Whether leading whitespace on wrapped continuation rows is dropped (default `true`).
    #[must_use]
    pub const fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Scrolls the content: `rows` visual rows down, `columns` columns right (no-wrap mode only).
    #[must_use]
    pub const fn scroll(mut self, rows: u16, columns: u16) -> Self {
        self.scroll_rows = rows;
        self.scroll_columns = columns;
        self
    }

    /// Returns how many visual rows the text needs at `width` columns (for sizing / scrollbars).
    #[must_use]
    pub fn line_count(&self, width: u16) -> usize {
        if width == 0 {
            return 0;
        }
        let mut total = 0_usize;
        for line in self.text.lines() {
            total = total.saturating_add(self.rows_in_line(line, width));
        }
        total
    }

    fn rows_in_line(&self, line: &Line<'_>, width: u16) -> usize {
        if self.wrap == WrapMode::None || line.is_empty() {
            return 1;
        }
        let mut cursor = Cursor::default();
        let mut rows = 0_usize;
        loop {
            if rows > 0 && self.trim {
                cursor = skip_whitespace(line.as_spans(), cursor);
            }
            let mut probe = cursor;
            if next_grapheme(line.as_spans(), &mut probe).is_none() {
                return rows.max(1);
            }
            let (end, _) = measure_row(line.as_spans(), cursor, width, self.wrap);
            rows = rows.saturating_add(1);
            if end == cursor {
                return rows;
            }
            cursor = end;
        }
    }

    /// Draws one visual row (`start..end` cursors) at `row`, honoring alignment.
    #[expect(clippy::too_many_arguments, reason = "internal row painter")]
    fn paint_row(
        &self,
        buffer: &mut Buffer,
        line: &Line<'_>,
        start: Cursor,
        end: Cursor,
        used: u16,
        area: Rect,
        row: RowIndex,
        skip_columns: u16,
    ) {
        let alignment = if line.text_alignment() == TextAlignment::Left {
            self.alignment
        } else {
            line.text_alignment()
        };
        let free = area.width().get().saturating_sub(used);
        let shift = match alignment {
            TextAlignment::Left => 0,
            TextAlignment::Center => free / 2,
            TextAlignment::Right => free,
        };
        let right_edge = area.right();
        let mut column = area.x().saturating_add(shift);
        let mut cursor = start;
        let mut skipped: u16 = 0;
        while cursor != end && column < right_edge {
            let Some((grapheme, span)) = next_grapheme(line.as_spans(), &mut cursor) else {
                break;
            };
            // Stop once we pass the end cursor (the row boundary can fall mid-span).
            if cursor.span > end.span || (cursor.span == end.span && cursor.byte > end.byte) {
                break;
            }
            let columns = grapheme_width(grapheme);
            if skipped < skip_columns {
                skipped = skipped.saturating_add(columns);
                continue;
            }
            let style = line.base_style().patch(span.style());
            column = buffer.set_str_clipped(column, row, grapheme, right_edge, style);
        }
    }
}

impl Widget for Paragraph<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        buffer.set_style(area, self.style);
        let height = usize::from(area.height().get());
        let width = area.width().get();
        let first_visible = usize::from(self.scroll_rows);
        let mut visual_row: usize = 0;

        for line in self.text.lines() {
            if visual_row >= first_visible.saturating_add(height) {
                break;
            }
            let spans = line.as_spans();
            let mut cursor = Cursor::default();
            let mut rows_in_this_line = 0_usize;
            loop {
                if rows_in_this_line > 0 && self.wrap != WrapMode::None && self.trim {
                    cursor = skip_whitespace(spans, cursor);
                }
                let mut probe = cursor;
                let has_content = next_grapheme(spans, &mut probe).is_some();
                if !has_content && rows_in_this_line > 0 {
                    break;
                }
                let row_limit = if self.wrap == WrapMode::None {
                    u16::MAX
                } else {
                    width
                };
                let (end, used) = measure_row(spans, cursor, row_limit, self.wrap);

                if visual_row >= first_visible {
                    let offset = u16::try_from(visual_row - first_visible).unwrap_or(u16::MAX);
                    let row = RowIndex::new(area.y().get().saturating_add(offset));
                    let skip_columns = if self.wrap == WrapMode::None {
                        self.scroll_columns
                    } else {
                        0
                    };
                    if row < area.bottom() {
                        let visible_used = used.saturating_sub(skip_columns);
                        self.paint_row(
                            buffer,
                            line,
                            cursor,
                            end,
                            visible_used,
                            area,
                            row,
                            skip_columns,
                        );
                    }
                }
                visual_row = visual_row.saturating_add(1);
                rows_in_this_line = rows_in_this_line.saturating_add(1);

                if self.wrap == WrapMode::None || end == cursor || !has_content {
                    break;
                }
                cursor = end;
                if visual_row >= first_visible.saturating_add(height) {
                    break;
                }
            }
        }
    }
}

impl<'a> From<&'a Paragraph<'a>> for Paragraph<'a> {
    fn from(value: &'a Paragraph<'a>) -> Self {
        value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_lines(paragraph: Paragraph<'_>, width: u16, height: u16) -> Vec<String> {
        let area = Rect::from_xywh(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        paragraph.render(area, &mut buffer);
        buffer.to_visual_lines()
    }

    #[test]
    fn word_wrap_breaks_at_spaces_and_trims_continuations() {
        let lines = render_to_lines(
            Paragraph::new("alpha beta gamma").wrap(WrapMode::Word),
            8,
            3,
        );
        assert_eq!(lines, vec!["alpha   ", "beta    ", "gamma   "]);
    }

    #[test]
    fn long_word_falls_back_to_character_break() {
        let lines = render_to_lines(Paragraph::new("abcdefghij").wrap(WrapMode::Word), 4, 3);
        assert_eq!(lines, vec!["abcd", "efgh", "ij  "]);
    }

    #[test]
    fn char_wrap_and_line_count_agree() {
        let paragraph = Paragraph::new("one two three four").wrap(WrapMode::Char);
        assert_eq!(paragraph.line_count(6), 3);
        let word = Paragraph::new("one two three four").wrap(WrapMode::Word);
        assert_eq!(word.line_count(9), 3);
        assert_eq!(word.line_count(10), 2);
    }

    #[test]
    fn scroll_skips_rows_and_columns() {
        let rows = render_to_lines(Paragraph::new("a\nb\nc\nd").scroll(2, 0), 3, 2);
        assert_eq!(rows, vec!["c  ", "d  "]);
        let columns = render_to_lines(Paragraph::new("abcdef").scroll(0, 2), 4, 1);
        assert_eq!(columns, vec!["cdef"]);
    }

    #[test]
    fn right_alignment_and_zero_sizes_do_not_panic() {
        let lines = render_to_lines(Paragraph::new("ab").alignment(TextAlignment::Right), 5, 1);
        assert_eq!(lines, vec!["   ab"]);
        let area = Rect::from_xywh(0, 0, 0, 0);
        let mut buffer = Buffer::empty(area);
        Paragraph::new("x")
            .wrap(WrapMode::Word)
            .render(area, &mut buffer);
        assert_eq!(Paragraph::new("x").wrap(WrapMode::Word).line_count(0), 0);
    }
}
