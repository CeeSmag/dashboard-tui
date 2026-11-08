//! Ergonomic prelude re-exporting the most commonly used types and traits in `dashboard-tui`.
//!
//! # Examples
//!
//! ```
//! use dashboard_tui::prelude::*;
//!
//! let area = Rect::from_size(TerminalWidth::new(80), TerminalHeight::new(24));
//! let buf = Buffer::empty(area);
//! assert_eq!(buf.area(), area);
//! ```

pub use crate::backend::{
    Backend, CrosstermBackend, ProgressBarState, TerminalCapabilities, TestBackend,
    format_hyperlink, format_hyperlink_with_id,
};
pub use crate::components::{
    DualWaveform, GradientMeter, GraphSymbolMode, HalfBlockQr, QrCodeMatrix, QrErrorCorrection,
    QrWidget, RateHistory, SingleWaveform, SliderBar, TimeSeries, WaveScaler, WaveScalerConfig,
    format_bytes, format_rate, format_rate_short, resample_slice_into, resample_slice_scaled_into,
    spinner_frame,
};
pub use crate::core::{
    Buffer, Cell, Color, ColorDepth, ColorPalette, ColumnIndex, Constraint, GradientLut, KeyHint,
    Layout, LayoutDirection, Line, Modifiers, PrioritySegment, Rect, RowIndex, Spacing4D, Span,
    StackStr, Style, TerminalBackground, TerminalHeight, TerminalWidth, TextAlignment,
};
pub use crate::dashboard::{
    AddonOptionEntry, AddonPageEntry, AddonRegistry, AddonSectionEntry, DashboardBackend,
    DashboardEngine, EngineAction, EventState, IntoKeyCode, IntoKeyPress, KeyBindings, KeyCode,
    KeyPress, Modal, ModalContext, ModalResult, OptionOutcome, OptionSpec, Page, PageSubView,
    Section, SectionFocusStyle, SectionLayoutMode, SectionRole, SectionVisibility, SectionWidget,
    SpacingConfig, SpacingLayer, SubView, SubViewAction, SubViewResult,
};
pub use crate::error::TuiError;
pub use crate::terminal::{
    TerminalSessionConfig, TerminalSessionGuard, install_panic_restoration_hook,
    restore_terminal_emergency,
};
pub use crate::universal::{
    ColorThemeModal, ConfirmModal, EditModal, LogBuffer, LogEntry, LogLevel, LogViewSubView,
    NoticeModal, SectionGapModal, SettingField, SettingsModal, StatusPanel, StatusRow, StatusValue,
    ThroughputPanel, TypedModal, color_theme_option, section_gap_option,
};
pub use crate::widgets::{
    Axis, Bar, BarChart, Calendar, Canvas, Chart, ChoiceRow, Clear, Dataset, Gauge, GraphType,
    Hyperlink, LabeledDivider, LineGauge, List, ListItem, ListState, Marker, MenuItem, MenuList,
    Paragraph, Row, Shape, Sparkline, SubCellScrollbar, TabPanel, Table, TableColumn, TableState,
    Tabs, TextInput, TextInputState, VerticalScrollState, WeekStart, WeightedTable, WrapMode,
};
