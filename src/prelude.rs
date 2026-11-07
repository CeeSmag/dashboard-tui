//! Ergonomic prelude re-exporting the most commonly used types and traits in `tui-rs`.
//!
//! # Examples
//!
//! ```
//! use tui_rs::prelude::*;
//!
//! let area = Rect::from_size(TerminalWidth::new(80), TerminalHeight::new(24));
//! let buf = Buffer::empty(area);
//! assert_eq!(buf.area(), area);
//! ```

pub use crate::backend::{
    Backend, CrosstermBackend, ProgressBarState, TerminalCapabilities, TestBackend,
};
pub use crate::components::{
    format_bytes, format_rate, format_rate_short, resample_slice_into,
    resample_slice_scaled_into, spinner_frame, DualWaveform, GradientMeter, GraphSymbolMode,
    HalfBlockQr, SingleWaveform, SliderBar, TimeSeries, WaveScaler, WaveScalerConfig,
};
pub use crate::core::{
    Buffer, Cell, Color, ColorDepth, ColorPalette, ColumnIndex, Constraint, GradientLut, KeyHint,
    Layout, LayoutDirection, Line, Modifiers, PrioritySegment, Rect, RowIndex, Spacing4D, Span,
    StackStr, Style, TerminalHeight, TerminalWidth, TextAlignment,
};
pub use crate::dashboard::{
    AddonOptionEntry, AddonPageEntry, AddonRegistry, AddonSectionEntry, DashboardBackend,
    DashboardEngine, EngineAction, EventState, IntoKeyCode, KeyBindings, KeyCode, KeyPress, Modal,
    ModalContext, ModalResult, OptionOutcome, OptionSpec, Page, Section, SectionFocusStyle,
    SectionLayoutMode, SectionRole, SectionVisibility, SectionWidget, SpacingConfig, SpacingLayer,
    SubView, SubViewResult,
};
pub use crate::error::TuiError;
pub use crate::terminal::{
    install_panic_restoration_hook, restore_terminal_emergency, TerminalSessionConfig,
    TerminalSessionGuard,
};
pub use crate::universal::{
    color_theme_option, section_gap_option, ColorThemeModal, ConfirmModal, EditModal, NoticeModal,
    SectionGapModal, TypedModal,
};
pub use crate::widgets::{
    ChoiceRow, LabeledDivider, MenuItem, MenuList, SubCellScrollbar, TabPanel, TableColumn,
    TextInput, TextInputState, VerticalScrollState, WeightedTable,
};
