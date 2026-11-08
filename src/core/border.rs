//! Box-drawing border glyph merging (`merge_border_symbols`).
//!
//! When two bordered regions touch, drawing one border on top of the other normally leaves
//! a broken join (a `│` stamped over a `─`). Merging combines the arms of both glyphs into
//! the right junction (`┬`, `├`, `┼`, …). Light (including rounded corners), heavy and
//! double lines merge with their own kind; mixed line weights keep the new glyph.

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::style::Style;

/// Weight of a box-drawing line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineWeight {
    Light,
    Heavy,
    Double,
}

// Glyph per arm-mask: bit 0 = up, bit 1 = right, bit 2 = down, bit 3 = left. Index 0 has no glyph.
const LIGHT: [char; 16] = [
    ' ', '╵', '╶', '└', '╷', '│', '┌', '├', '╴', '┘', '─', '┴', '┐', '┤', '┬', '┼',
];
const HEAVY: [char; 16] = [
    ' ', '╹', '╺', '┗', '╻', '┃', '┏', '┣', '╸', '┛', '━', '┻', '┓', '┫', '┳', '╋',
];
// Double lines have no single-arm stubs; those masks map to a space.
const DOUBLE: [char; 16] = [
    ' ', ' ', ' ', '╚', ' ', '║', '╔', '╠', ' ', '╝', '═', '╩', '╗', '╣', '╦', '╬',
];

fn decode(symbol: char) -> Option<(LineWeight, u8)> {
    let tables: [(LineWeight, &[char; 16]); 3] = [
        (LineWeight::Light, &LIGHT),
        (LineWeight::Heavy, &HEAVY),
        (LineWeight::Double, &DOUBLE),
    ];
    // Rounded corners behave as light corners.
    let symbol = match symbol {
        '╭' => '┌',
        '╮' => '┐',
        '╰' => '└',
        '╯' => '┘',
        other => other,
    };
    for (weight, table) in tables {
        if let Some(mask) = table.iter().position(|&glyph| glyph == symbol) {
            if mask != 0 && symbol != ' ' {
                return Some((weight, u8::try_from(mask).unwrap_or(0)));
            }
        }
    }
    None
}

fn encode(weight: LineWeight, mask: u8) -> Option<char> {
    let table = match weight {
        LineWeight::Light => &LIGHT,
        LineWeight::Heavy => &HEAVY,
        LineWeight::Double => &DOUBLE,
    };
    let glyph = *table.get(usize::from(mask))?;
    (glyph != ' ').then_some(glyph)
}

/// Merges two box-drawing glyphs into the glyph that has the arms of both.
///
/// Returns `None` when either symbol is not a single box-drawing glyph, when the two use
/// different line weights, or when no glyph exists for the combination.
///
/// # Examples
///
/// ```
/// use tui_rs::core::merge_border_symbols;
///
/// assert_eq!(merge_border_symbols("─", "│"), Some('┼'));
/// assert_eq!(merge_border_symbols("┌", "┐"), Some('┬'));
/// assert_eq!(merge_border_symbols("╭", "├"), Some('├'));
/// assert_eq!(merge_border_symbols("═", "║"), Some('╬'));
/// assert_eq!(merge_border_symbols("─", "━"), None); // different weights
/// assert_eq!(merge_border_symbols("a", "│"), None);
/// ```
#[must_use]
pub fn merge_border_symbols(existing: &str, incoming: &str) -> Option<char> {
    let mut existing_chars = existing.chars();
    let mut incoming_chars = incoming.chars();
    let (old, new) = (existing_chars.next()?, incoming_chars.next()?);
    if existing_chars.next().is_some() || incoming_chars.next().is_some() {
        return None;
    }
    let (old_weight, old_mask) = decode(old)?;
    let (new_weight, new_mask) = decode(new)?;
    if old_weight != new_weight {
        return None;
    }
    let merged = old_mask | new_mask;
    encode(old_weight, merged)
}

impl Buffer {
    /// Draws a border glyph at `(column, row)`, merging with a border glyph already there.
    ///
    /// If the existing cell holds a compatible box-drawing glyph the result is the combined
    /// junction; otherwise `symbol` simply overwrites the cell. Out-of-bounds writes are ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style};
    ///
    /// let mut buffer = Buffer::empty(Rect::from_xywh(0, 0, 3, 1));
    /// buffer.set_border_symbol(ColumnIndex(1), RowIndex(0), "─", Style::DEFAULT);
    /// buffer.set_border_symbol(ColumnIndex(1), RowIndex(0), "│", Style::DEFAULT);
    /// assert_eq!(buffer.to_visual_lines(), vec![" ┼ "]);
    /// ```
    pub fn set_border_symbol(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        symbol: &str,
        style: Style,
    ) {
        let merged = self
            .cell_at(column, row)
            .and_then(|cell| merge_border_symbols(cell.symbol(), symbol));
        let mut scratch = [0_u8; 4];
        let glyph: &str = match merged {
            Some(joined) => joined.encode_utf8(&mut scratch),
            None => symbol,
        };
        let _ = self.set_string(column, row, glyph, style, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging_any_two_glyphs_of_one_weight_unions_their_arms() {
        for table in [&LIGHT, &HEAVY, &DOUBLE] {
            for (first_mask, &first) in table.iter().enumerate().filter(|(_, g)| **g != ' ') {
                for (second_mask, &second) in table.iter().enumerate().filter(|(_, g)| **g != ' ') {
                    let mut a = [0_u8; 4];
                    let mut b = [0_u8; 4];
                    let merged =
                        merge_border_symbols(first.encode_utf8(&mut a), second.encode_utf8(&mut b));
                    let expected_mask = first_mask | second_mask;
                    // Double lines have no stub glyphs, so some unions have no glyph at all.
                    if let Some(glyph) = merged {
                        assert_eq!(table[expected_mask], glyph);
                    } else {
                        assert_eq!(table[expected_mask], ' ');
                    }
                }
            }
        }
    }

    #[test]
    fn rounded_corners_merge_like_light_corners() {
        assert_eq!(merge_border_symbols("╰", "╯"), Some('┴'));
        assert_eq!(merge_border_symbols("╭", "╮"), Some('┬'));
    }
}
