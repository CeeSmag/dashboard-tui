//! Tier 1 Portable Dashboard Engine (`src/dashboard/`).
//!
//! Provides the complete domain-agnostic dashboard framework:
//! - [`DashboardBackend`]: 3-method contract between Tier 1 and the host application.
//! - [`DashboardEngine`]: Multi-page, multi-section layout, focus, and modal/sub-view
//!   orchestrator.
//! - [`Page`] & [`SectionLayoutMode`]: 6 responsive section layout modes (`Auto`,
//!   `Columns`, `Rows`, `OneTopTwoBottom`, `TwoTopOneBottom`, `Grid2x2`).
//! - [`Section`], [`SectionWidget`], [`OptionSpec`], [`OptionOutcome`]: Display
//!   telemetry widgets and numbered (`[1]`..`[99]`) menu options.
//! - [`SpacingConfig`] & [`SpacingLayer`]: 4-layer hierarchical spacing system.
//! - [`Modal`], [`ModalContext`], [`ModalResult`], [`splice_modal_overlay`]:
//!   Centered modal overlays with dimmed backdrop splicing.
//! - [`SubView`], [`SubViewResult`]: Full-area drill-down view stack.
//! - [`AddonRegistry`], [`AddonPageEntry`], [`AddonSectionEntry`], [`AddonOptionEntry`]:
//!   Zero-edit declarative add-on wiring.

pub mod backend_trait;
pub mod chrome;
pub mod engine;
pub mod key_router;
pub mod modal;
pub mod navigation;
pub mod page;
pub mod page_sub_view;
pub mod registry;
pub mod section;
pub mod spacing;
pub mod sub_view;

pub use backend_trait::DashboardBackend;
pub use chrome::{render_footer_bar, render_header_bar};
pub use engine::DashboardEngine;
pub use key_router::{
    EngineAction, EventState, IntoKeyCode, IntoKeyPress, KeyBindings, KeyCode, KeyPress,
};
pub use modal::{
    Modal, ModalContext, ModalResult, dim_buffer_backdrop, splice_modal_overlay,
    splice_modal_overlay_with_backend,
};
pub use navigation::{DigitDispatch, MULTI_DIGIT_TIMEOUT, PrefixJumpState};
pub use page::{Page, ResolvedSectionRect, SectionLayoutMode};
pub use page_sub_view::PageSubView;
pub use registry::{AddonOptionEntry, AddonPageEntry, AddonRegistry, AddonSectionEntry};
pub use section::{
    ActionFn, BadgeFn, EnabledFn, OptionOutcome, OptionSpec, Section, SectionFocusStyle,
    SectionRole, SectionVisibility, SectionWidget,
};
pub use spacing::{SpacingConfig, SpacingLayer};
pub use sub_view::{SubView, SubViewAction, SubViewResult};
