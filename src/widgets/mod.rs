//! Composable structural TUI widgets (`TabPanel`, `LabeledDivider`, `SubCellScrollbar`,
//! `WeightedTable`, `MenuList`, `ChoiceRow`, `TextInput`, `Hyperlink`).

pub mod bar_chart;
pub mod calendar;
pub mod canvas;
pub mod chart;
pub mod choice_row;
pub mod clear;
pub mod divider;
pub mod gauge;
pub mod hyperlink;
pub mod list;
pub mod menu_list;
pub mod panel;
pub mod paragraph;
pub mod plot;
pub mod scrollbar;
pub mod sparkline;
pub mod table;
pub mod tabs;
pub mod text_input;
pub mod weighted_table;

pub use bar_chart::{Bar, BarChart};
pub use calendar::{Calendar, WeekStart, days_in_month, first_weekday_of_month, is_leap_year};
pub use canvas::{Canvas, Shape};
pub use chart::{Axis, Chart, Dataset, GraphType};
pub use choice_row::ChoiceRow;
pub use clear::Clear;
pub use divider::LabeledDivider;
pub use gauge::{Gauge, LineGauge};
pub use hyperlink::Hyperlink;
pub use list::{List, ListItem, ListState};
pub use menu_list::{MenuItem, MenuList};
pub use panel::{TabPanel, format_superscript_number};
pub use paragraph::{Paragraph, WrapMode};
pub use plot::{DotPlotter, Marker};
pub use scrollbar::{SubCellScrollbar, VerticalScrollState};
pub use sparkline::Sparkline;
pub use table::{Row, Table, TableState};
pub use tabs::Tabs;
pub use text_input::{TextInput, TextInputState};
pub use weighted_table::{MAX_TABLE_COLUMNS, TableColumn, WeightedTable};

/// Converts a non-negative `f64` (clamped to `0.0..=65535.0`, `NaN` → 0) to `u16`, rounding down.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped into the u16 range first; NaN maps to 0"
)]
pub(crate) fn floor_to_u16(value: f64) -> u16 {
    if value.is_nan() {
        0
    } else {
        value.clamp(0.0, f64::from(u16::MAX)).floor() as u16
    }
}

/// Like [`floor_to_u16`] but rounds to the nearest integer.
pub(crate) fn round_to_u16(value: f64) -> u16 {
    floor_to_u16(value.round())
}
