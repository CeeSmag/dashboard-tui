//! Composable structural TUI widgets (`TabPanel`, `LabeledDivider`, `SubCellScrollbar`,
//! `WeightedTable`, `MenuList`, `ChoiceRow`, `TextInput`).

pub mod choice_row;
pub mod divider;
pub mod menu_list;
pub mod panel;
pub mod scrollbar;
pub mod text_input;
pub mod weighted_table;

pub use choice_row::ChoiceRow;
pub use divider::LabeledDivider;
pub use menu_list::{MenuItem, MenuList};
pub use panel::{format_superscript_number, TabPanel};
pub use scrollbar::{SubCellScrollbar, VerticalScrollState};
pub use text_input::{TextInput, TextInputState};
pub use weighted_table::{TableColumn, WeightedTable, MAX_TABLE_COLUMNS};
