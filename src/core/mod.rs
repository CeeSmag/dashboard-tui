//! Core zero-dependency TUI primitives: coordinates, geometry, colors, styles, palettes,
//! UAX #29 graphemes, double-buffered cell grids, and constraint layout solvers.

pub mod buffer;
pub mod cell;
pub mod color;
pub mod coordinates;
pub mod diff;
pub mod grapheme;
pub mod layout;
pub mod palette;
pub mod priority_strip;
pub mod rect;
pub mod style;
pub mod text;

pub use self::buffer::{Buffer, StackStr};
pub use self::cell::Cell;
pub use self::color::{Color, ColorDepth, rgb_to_ansi16, rgb_to_ansi256};
pub use self::coordinates::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
pub use self::diff::{CellDiff, DiffIter, diff_buffers};
pub use self::grapheme::{
    WrapLinesIter, clip_from_left, clip_to_width, display_width, grapheme_width,
    truncate_with_ellipsis_parts, wrap_lines_iter,
};
pub use self::layout::{Constraint, Layout, LayoutDirection, split_equal_spans};
pub use self::palette::{ColorPalette, GradientLut, TerminalBackground};
pub use self::priority_strip::{
    KeyHint, PrioritySegment, fit_hint_indices, fit_segment_indices, render_fitted_hints,
    render_fitted_segments,
};
pub use self::rect::{Rect, Spacing4D};
pub use self::style::{Modifiers, Style};
pub use self::text::{Line, Span, TextAlignment};
