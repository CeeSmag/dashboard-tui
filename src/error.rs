//! Granular, actionable domain error types for `tui-rs`.
//!
//! All error enums are marked `#[non_exhaustive]` to preserve `SemVer` compatibility
//! across minor releases.

use thiserror::Error;

/// Top-level error type returned by `tui-rs` terminal, layout, and dashboard operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TuiError {
    /// Underlying operating system or terminal I/O failure.
    #[error("terminal I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Viewport or terminal dimensions are invalid for the requested operation.
    #[error("invalid terminal dimensions (width={width}, height={height})")]
    InvalidDimensions {
        /// Measured or requested width in columns.
        width: u16,
        /// Measured or requested height in rows.
        height: u16,
    },

    /// A coordinate fell outside the bounds of a [`crate::core::Rect`] or [`crate::core::Buffer`].
    #[error("coordinate (col={column}, row={row}) is out of bounds for rect ({width}x{height})")]
    OutOfBounds {
        /// Target column index.
        column: u16,
        /// Target row index.
        row: u16,
        /// Width of the bounding rectangle.
        width: u16,
        /// Height of the bounding rectangle.
        height: u16,
    },

    /// A hex color string could not be parsed into RGB components.
    #[error("invalid hex color string '{input}': expected #RRGGBB or RRGGBB")]
    InvalidHexColor {
        /// The malformed color input string.
        input: String,
    },

    /// A dashboard action or configuration update failed in the backend.
    #[error("dashboard backend error: {message}")]
    Backend {
        /// Human-readable error message from the backend.
        message: String,
    },
}
