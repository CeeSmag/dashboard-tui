//! Unicode Standard Annex #29 grapheme cluster segmentation and East Asian display width layout math.
//!
//! Never slices or indexes user-facing strings by raw byte offset or `char` count.
//! All functions operate on extended grapheme clusters (`unicode_segmentation`) and
//! visual column widths (`unicode_width`) with zero heap allocation where possible.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::core::coordinates::TerminalWidth;

/// Computes the visual terminal column width of `text` using East Asian width rules.
///
/// Saturates at `u16::MAX`.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{display_width, TerminalWidth};
///
/// assert_eq!(display_width("VaultDrive"), TerminalWidth::new(10));
/// assert_eq!(display_width("日本"), TerminalWidth::new(4));
/// ```
#[must_use]
pub fn display_width(text: &str) -> TerminalWidth {
    let mut total = 0u16;
    for grapheme in text.graphemes(true) {
        let w = grapheme_width(grapheme);
        total = total.saturating_add(w);
    }
    TerminalWidth(total)
}

/// Computes the visual column width of a single extended grapheme cluster (`0`, `1`, or `2`).
///
/// Control characters (`\n`, `\r`, `\t`, `\0`) report width `0` so they never corrupt grid math.
///
/// # Examples
///
/// ```
/// use tui_rs::core::grapheme_width;
///
/// assert_eq!(grapheme_width("a"), 1);
/// assert_eq!(grapheme_width("界"), 2);
/// assert_eq!(grapheme_width("\n"), 0);
/// ```
#[must_use]
pub fn grapheme_width(grapheme: &str) -> u16 {
    if grapheme.is_empty() || grapheme.starts_with(|c: char| c.is_control()) {
        return 0;
    }
    let width = UnicodeWidthStr::width(grapheme);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to at most 2 columns per grapheme"
    )]
    let clamped = width.min(2) as u16;
    clamped
}

/// Clips `text` from the left so its visual width does not exceed `max_width` columns.
///
/// Returns a borrowed sub-slice `&str` with zero heap allocation (equivalent to `btop`'s `uresize`).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{clip_to_width, TerminalWidth};
///
/// assert_eq!(clip_to_width("Hello World", TerminalWidth(5)), "Hello");
/// // Wide 2-column CJK characters will not split mid-character:
/// assert_eq!(clip_to_width("日本", TerminalWidth(3)), "日");
/// ```
#[must_use]
pub fn clip_to_width(text: &str, max_width: TerminalWidth) -> &str {
    let limit = max_width.as_u16();
    if limit == 0 || text.is_empty() {
        return "";
    }
    let mut accumulated = 0u16;
    let mut byte_end = 0usize;

    for (byte_idx, grapheme) in text.grapheme_indices(true) {
        let gw = grapheme_width(grapheme);
        if accumulated.saturating_add(gw) > limit {
            break;
        }
        accumulated = accumulated.saturating_add(gw);
        byte_end = byte_idx.saturating_add(grapheme.len());
    }

    &text[..byte_end]
}

/// Clips `text` from the right (keeping the trailing graphemes) so its visual width is at most `max_width`.
///
/// Equivalent to `btop`'s `luresize` used in horizontal text-input scrolling.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{clip_from_left, TerminalWidth};
///
/// assert_eq!(clip_from_left("Hello World", TerminalWidth(5)), "World");
/// ```
#[must_use]
pub fn clip_from_left(text: &str, max_width: TerminalWidth) -> &str {
    let limit = max_width.as_u16();
    if limit == 0 || text.is_empty() {
        return "";
    }
    let mut accumulated = 0u16;
    let mut byte_start = text.len();

    for (byte_idx, grapheme) in text.grapheme_indices(true).rev() {
        let gw = grapheme_width(grapheme);
        if accumulated.saturating_add(gw) > limit {
            break;
        }
        accumulated = accumulated.saturating_add(gw);
        byte_start = byte_idx;
    }

    &text[byte_start..]
}

/// Computes the prefix of `text` and whether an ellipsis `"…"` is needed to fit within `max_width`.
///
/// Returns `(prefix_slice, needs_ellipsis, total_visual_width)`. Zero heap allocation.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{truncate_with_ellipsis_parts, TerminalWidth};
///
/// let (prefix, ellipsis, width) = truncate_with_ellipsis_parts("VaultDrive Service", TerminalWidth(10));
/// assert_eq!(prefix, "VaultDriv");
/// assert!(ellipsis);
/// assert_eq!(width, TerminalWidth(10));
/// ```
#[must_use]
pub fn truncate_with_ellipsis_parts(
    text: &str,
    max_width: TerminalWidth,
) -> (&str, bool, TerminalWidth) {
    let full_width = display_width(text);
    if full_width <= max_width {
        return (text, false, full_width);
    }
    if max_width.is_zero() {
        return ("", false, TerminalWidth::ZERO);
    }
    if max_width.as_u16() == 1 {
        return ("", true, TerminalWidth(1));
    }
    let prefix = clip_to_width(text, max_width.saturating_sub(1));
    let prefix_w = display_width(prefix);
    (prefix, true, prefix_w.saturating_add(1))
}

/// Zero-allocation word-wrapping iterator that yields borrowed line slices `&str` fitting within `max_width`.
///
/// Preserves explicit `\n` line breaks and breaks long words at grapheme boundaries when a single word
/// exceeds `max_width`.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{wrap_lines_iter, TerminalWidth};
///
/// let lines: Vec<&str> = wrap_lines_iter("Mount virtual drive now", TerminalWidth(12)).collect();
/// assert_eq!(lines, vec!["Mount", "virtual", "drive now"]);
/// ```
#[must_use]
pub fn wrap_lines_iter(text: &str, max_width: TerminalWidth) -> WrapLinesIter<'_> {
    WrapLinesIter {
        remaining: text,
        max_width,
        finished: text.is_empty() || max_width.is_zero(),
    }
}

/// Iterator returned by [`wrap_lines_iter`].
#[derive(Debug, Clone)]
pub struct WrapLinesIter<'a> {
    remaining: &'a str,
    max_width: TerminalWidth,
    finished: bool,
}

impl<'a> Iterator for WrapLinesIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Handle explicit newline first
        let (paragraph, after_newline) = match self.remaining.find('\n') {
            Some(idx) => (&self.remaining[..idx], Some(&self.remaining[idx + 1..])),
            None => (self.remaining, None),
        };

        let trimmed_para = paragraph.trim_end_matches('\r');
        let para_width = display_width(trimmed_para);
        if para_width <= self.max_width {
            if let Some(rest) = after_newline {
                self.remaining = rest;
                if self.remaining.is_empty() {
                    self.finished = true;
                }
            } else {
                self.remaining = "";
                self.finished = true;
            }
            return Some(trimmed_para);
        }

        // Find the last whitespace break within max_width
        let clipped = clip_to_width(trimmed_para, self.max_width);
        if let Some(space_idx) = clipped.rfind(char::is_whitespace) {
            if space_idx > 0 {
                let line = clipped[..space_idx].trim_end();
                let rest_in_para = trimmed_para[space_idx..].trim_start_matches([' ', '\t']);
                let offset_in_remaining =
                    trimmed_para.len().saturating_sub(rest_in_para.len());
                self.remaining = &self.remaining[offset_in_remaining..];
                return Some(line);
            }
        }

        // Hard break at grapheme boundary when a single word exceeds max_width
        let split_len = clipped.len();
        if split_len == 0 {
            // Even the first grapheme is wider than max_width (e.g. 2-col CJK in 1-col rect):
            // advance past one grapheme to guarantee forward progress.
            if let Some((_, first_g)) = self.remaining.grapheme_indices(true).next() {
                self.remaining = &self.remaining[first_g.len()..];
                if self.remaining.is_empty() {
                    self.finished = true;
                }
                return Some("");
            }
            self.finished = true;
            return None;
        }

        let line = &trimmed_para[..split_len];
        let rest_after = self.remaining[split_len..].trim_start_matches([' ', '\t']);
        self.remaining = rest_after;
        if self.remaining.is_empty() {
            self.finished = true;
        }
        Some(line)
    }
}
