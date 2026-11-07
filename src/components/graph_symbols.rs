//! Multi-mode sub-cell graph symbol lookup tables (`Braille`, `Block`, `Tty`).
//!
//! Synthesizes the 5×5 `(left_dots, right_dots)` lookup tables from `btop`
//! (`btop_draw.cpp`) and `ref/dashboard/de_monitors.go`. Every cell encodes
//! two adjacent horizontal time buckets (`0..=4` vertical sub-steps each),
//! doubling the horizontal time resolution of terminal graphs.

/// Rendering glyph set for high-resolution sub-cell graphs.
///
/// # Examples
///
/// ```
/// use tui_rs::components::GraphSymbolMode;
///
/// let mode = GraphSymbolMode::default();
/// assert_eq!(mode, GraphSymbolMode::Braille);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GraphSymbolMode {
    /// Unicode Braille patterns (`U+2800..U+28FF`), 2×4 dots per cell.
    #[default]
    Braille,
    /// Unicode quadrant/half-block glyphs (`▁▂▃▄▅▆▇█`), 2×2 effective resolution.
    Block,
    /// Safe CP437 / Linux TTY shade blocks (`' '`, `'░'`, `'▒'`, `'█'`).
    Tty,
}

/// 5×5 Braille lookup table growing **upward** from the bottom of the cell (`[left][right]`).
const BRAILLE_UP: [[&str; 5]; 5] = [
    [" ", "⢀", "⢠", "⢰", "⢸"],
    ["⡀", "⣀", "⣠", "⣰", "⣸"],
    ["⡄", "⣄", "⣤", "⣴", "⣼"],
    ["⡆", "⣆", "⣦", "⣶", "⣾"],
    ["⡇", "⣇", "⣧", "⣷", "⣿"],
];

/// 5×5 Braille lookup table growing **downward** from the top of the cell (`[left][right]`).
const BRAILLE_DOWN: [[&str; 5]; 5] = [
    [" ", "⠈", "⠘", "⠸", "⢸"],
    ["⠁", "⠉", "⠙", "⠹", "⢹"],
    ["⠃", "⠋", "⠛", "⠻", "⢻"],
    ["⠇", "⠏", "⠟", "⠿", "⢿"],
    ["⡇", "⡏", "⡟", "⡿", "⣿"],
];

/// 5×5 Unicode Block lookup table growing **upward** (`[left][right]`).
const BLOCK_UP: [[&str; 5]; 5] = [
    [" ", "▗", "▗", "▐", "▐"],
    ["▖", "▄", "▄", "▟", "▟"],
    ["▖", "▄", "▄", "▟", "▟"],
    ["▌", "▙", "▙", "█", "█"],
    ["▌", "▙", "▙", "█", "█"],
];

/// 5×5 Unicode Block lookup table growing **downward** (`[left][right]`).
const BLOCK_DOWN: [[&str; 5]; 5] = [
    [" ", "▝", "▝", "▐", "▐"],
    ["▘", "▀", "▀", "▜", "▜"],
    ["▘", "▀", "▀", "▜", "▜"],
    ["▌", "▛", "▛", "█", "█"],
    ["▌", "▛", "▛", "█", "█"],
];

/// 5×5 TTY safe shade lookup table growing **upward** (`[left][right]`).
const TTY_UP: [[&str; 5]; 5] = [
    [" ", "░", "░", "▒", "▒"],
    ["░", "░", "▒", "▒", "█"],
    ["░", "▒", "▒", "▒", "█"],
    ["▒", "▒", "▒", "█", "█"],
    ["▒", "█", "█", "█", "█"],
];

/// 5×5 TTY safe shade lookup table growing **downward** (`[left][right]`).
const TTY_DOWN: [[&str; 5]; 5] = [
    [" ", "░", "░", "▒", "▒"],
    ["░", "░", "▒", "▒", "█"],
    ["░", "▒", "▒", "▒", "█"],
    ["▒", "▒", "▒", "█", "█"],
    ["▒", "█", "█", "█", "█"],
];

/// Clamps a dot count to the valid sub-cell range `0..=4`.
///
/// # Examples
///
/// ```
/// use tui_rs::components::clamp_dots;
///
/// assert_eq!(clamp_dots(0), 0);
/// assert_eq!(clamp_dots(3), 3);
/// assert_eq!(clamp_dots(9), 4);
/// ```
#[must_use]
#[inline]
pub const fn clamp_dots(dots: usize) -> usize {
    if dots > 4 {
        4
    } else {
        dots
    }
}

/// Returns the upward-growing Braille glyph for `(left_dots, right_dots)` in `0..=4`.
///
/// # Examples
///
/// ```
/// use tui_rs::components::braille_up_cell;
///
/// assert_eq!(braille_up_cell(0, 0), " ");
/// assert_eq!(braille_up_cell(4, 4), "⣿");
/// assert_eq!(braille_up_cell(1, 0), "⡀");
/// ```
#[must_use]
#[inline]
pub const fn braille_up_cell(left_dots: usize, right_dots: usize) -> &'static str {
    BRAILLE_UP[clamp_dots(left_dots)][clamp_dots(right_dots)]
}

/// Returns the downward-growing Braille glyph for `(left_dots, right_dots)` in `0..=4`.
///
/// # Examples
///
/// ```
/// use tui_rs::components::braille_down_cell;
///
/// assert_eq!(braille_down_cell(0, 0), " ");
/// assert_eq!(braille_down_cell(4, 4), "⣿");
/// assert_eq!(braille_down_cell(1, 0), "⠁");
/// ```
#[must_use]
#[inline]
pub const fn braille_down_cell(left_dots: usize, right_dots: usize) -> &'static str {
    BRAILLE_DOWN[clamp_dots(left_dots)][clamp_dots(right_dots)]
}

/// Returns the upward-growing graph glyph for the specified [`GraphSymbolMode`].
///
/// # Examples
///
/// ```
/// use tui_rs::components::{graph_up_cell, GraphSymbolMode};
///
/// assert_eq!(graph_up_cell(GraphSymbolMode::Braille, 4, 4), "⣿");
/// assert_eq!(graph_up_cell(GraphSymbolMode::Block, 4, 4), "█");
/// assert_eq!(graph_up_cell(GraphSymbolMode::Tty, 4, 4), "█");
/// ```
#[must_use]
#[inline]
pub const fn graph_up_cell(
    mode: GraphSymbolMode,
    left_dots: usize,
    right_dots: usize,
) -> &'static str {
    let l = clamp_dots(left_dots);
    let r = clamp_dots(right_dots);
    match mode {
        GraphSymbolMode::Braille => BRAILLE_UP[l][r],
        GraphSymbolMode::Block => BLOCK_UP[l][r],
        GraphSymbolMode::Tty => TTY_UP[l][r],
    }
}

/// Returns the downward-growing graph glyph for the specified [`GraphSymbolMode`].
///
/// # Examples
///
/// ```
/// use tui_rs::components::{graph_down_cell, GraphSymbolMode};
///
/// assert_eq!(graph_down_cell(GraphSymbolMode::Braille, 1, 1), "⠉");
/// assert_eq!(graph_down_cell(GraphSymbolMode::Block, 1, 1), "▀");
/// ```
#[must_use]
#[inline]
pub const fn graph_down_cell(
    mode: GraphSymbolMode,
    left_dots: usize,
    right_dots: usize,
) -> &'static str {
    let l = clamp_dots(left_dots);
    let r = clamp_dots(right_dots);
    match mode {
        GraphSymbolMode::Braille => BRAILLE_DOWN[l][r],
        GraphSymbolMode::Block => BLOCK_DOWN[l][r],
        GraphSymbolMode::Tty => TTY_DOWN[l][r],
    }
}

/// Converts a sample `value` in `0.0..=max_value` into total vertical sub-cell
/// dots (`0..=rows * 4`).
///
/// Any positive non-zero value produces at least `1` dot so low-level activity
/// remains visible rather than disappearing into a flat blank line.
///
/// # Examples
///
/// ```
/// use tui_rs::components::total_wave_dots;
///
/// assert_eq!(total_wave_dots(0.0, 100.0, 4), 0);
/// assert_eq!(total_wave_dots(0.1, 100.0, 4), 1);
/// assert_eq!(total_wave_dots(100.0, 100.0, 4), 16);
/// ```
#[must_use]
#[inline]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "clamped to 0..=rows*4 before casting to usize"
)]
pub fn total_wave_dots(value: f64, max_value: f64, rows: usize) -> usize {
    if rows == 0 || value <= 0.0 || max_value <= 0.0 {
        return 0;
    }
    let max_dots = rows.saturating_mul(4);
    if value >= max_value {
        return max_dots;
    }
    let scaled = ((value / max_value) * (max_dots as f64)).round() as usize;
    scaled.clamp(1, max_dots)
}

/// Computes the number of filled sub-cell dots (`0..=4`) within a single row
/// `row_from_base` (`0` = row closest to baseline, `rows - 1` = furthest row).
///
/// # Examples
///
/// ```
/// use tui_rs::components::row_wave_dots;
///
/// // 6 total dots across 2 rows: bottom row (0) has 4 dots, top row (1) has 2 dots.
/// assert_eq!(row_wave_dots(6, 0), 4);
/// assert_eq!(row_wave_dots(6, 1), 2);
/// ```
#[must_use]
#[inline]
pub const fn row_wave_dots(total_dots: usize, row_from_base: usize) -> usize {
    let row_base = row_from_base.saturating_mul(4);
    if total_dots <= row_base {
        0
    } else {
        clamp_dots(total_dots.saturating_sub(row_base))
    }
}
