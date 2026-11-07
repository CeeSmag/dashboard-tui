//! Tier 2 Universal Add-ons (`src/universal/`).
//!
//! Project-independent modal dialogs and plug-and-play settings options:
//! - [`EditModal`]: Single-line text or masked password input dialog.
//! - [`ConfirmModal`]: `[ Yes ]` / `[ No ]` confirmation dialog.
//! - [`TypedModal`]: Destructive action guard requiring an exact phrase to be typed.
//! - [`NoticeModal`]: Scrollable multi-line informational dialog with sub-cell scrollbar.
//! - [`SectionGapModal`] & [`section_gap_option`]: Interactive 4-layer spacing adjuster (`[19]`).
//! - [`ColorThemeModal`] & [`color_theme_option`]: Interactive 6-palette theme picker (`[20]`).

pub mod color_theme_modal;
pub mod confirm_modal;
pub mod edit_modal;
pub mod notice_modal;
pub mod section_gap_modal;
pub mod typed_modal;

pub use color_theme_modal::{color_theme_option, ColorThemeModal};
pub use confirm_modal::{ConfirmActionFn, ConfirmModal};
pub use edit_modal::{EditModal, EditSubmitFn};
pub use notice_modal::NoticeModal;
pub use section_gap_modal::{section_gap_option, SectionGapModal};
pub use typed_modal::TypedModal;
