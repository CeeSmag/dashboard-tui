//! `tui-rs`: Zero-allocation Terminal User Interface (TUI) and 3-Tier Portable
//! Dashboard Engine library in Rust.
//!
//! Architected according to `RULES.md` to power the full Rust rewrite of
//! `VaultDrive` (`/home/cee/Documents/vaultdrive-linux`) and reusable across any
//! interactive systems telemetry or management application.
//!
//! > **For AI Agents & Integrators**: Read `AI_GUIDE.md` in the repository root for the
//! > complete file-by-file navigation matrix and the canonical `src/tui/dashboard.rs` +
//! > `src/tui/addons/` architecture blueprint.
//!
//! # Architectural Layers
//!
//! 1. **Layer 0 ([`backend`])**: Low-level terminal I/O abstraction, DEC Mode 2026
//!    synchronized output, DEC Mode 1007 alternate scroll, DEC Mode 2027 grapheme
//!    clustering, OSC `9;4` progress reporting, Linux TTY capability detection,
//!    and headless [`backend::TestBackend`].
//! 2. **Layer 0.5 ([`terminal`])**: RAII [`terminal::TerminalSessionGuard`] with
//!    double-buffered cell diffing and composable panic hook restoration.
//! 3. **Layer 1 ([`core`])**: Domain coordinate newtypes, [`core::Rect`] & [`core::Spacing4D`]
//!    geometry, [`core::Buffer`] 2D cell grid, [`core::ColorPalette`] & 101-entry
//!    [`core::GradientLut`], Unicode grapheme clipping, iterative layout solver,
//!    and priority-based header/footer strip fitting.
//! 4. **Layer 2 ([`components`] & [`widgets`])**: `btop`-grade 5×5 sub-cell
//!    [`components::SingleWaveform`] and [`components::DualWaveform`] graphs,
//!    hysteresis [`components::WaveScaler`], [`components::GradientMeter`],
//!    [`components::SliderBar`], QR generator ([`components::QrCodeMatrix`], [`components::HalfBlockQr`],
//!    [`components::QrWidget`]), [`widgets::TabPanel`], [`widgets::Hyperlink`],
//!    [`widgets::MenuList`], [`widgets::WeightedTable`], and [`widgets::SubCellScrollbar`].
//! 5. **Layer 3 — Tier 1 ([`dashboard`])**: Domain-agnostic [`dashboard::DashboardEngine`]
//!    with 3-method [`dashboard::DashboardBackend`] contract, 4-layer [`dashboard::SpacingConfig`],
//!    6 [`dashboard::SectionLayoutMode`] presets, multi-digit `1`..`99` prefix jump,
//!    dimmed modal backdrop splicing, and zero-edit [`dashboard::AddonRegistry`].
//! 6. **Layer 4 — Tier 2 ([`universal`])**: Plug-and-play [`universal::EditModal`],
//!    [`universal::ConfirmModal`], [`universal::TypedModal`], [`universal::NoticeModal`],
//!    [`universal::SectionGapModal`] (`[19]`), and [`universal::ColorThemeModal`] (`[20]`).
//!
//! # Examples
//!
//! ```
//! use tui_rs::prelude::*;
//!
//! let backend = TestBackend::new(TerminalWidth::new(80), TerminalHeight::new(24));
//! let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
//! session.draw(|buffer, area| {
//!     let inner = TabPanel::new("VaultDrive").with_badge("MOUNTED").render(
//!         buffer,
//!         area,
//!         &ColorPalette::DEFAULT,
//!     );
//!     buffer.set_string(
//!         inner.x(),
//!         inner.y(),
//!         "Zero-allocation TUI Engine",
//!         Style::new().with_fg(Color::Green),
//!         inner.width().get(),
//!     );
//! })?;
//! # Ok::<(), TuiError>(())
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod backend;
pub mod components;
pub mod core;
pub mod dashboard;
pub mod error;
pub mod prelude;
pub mod terminal;
pub mod universal;
pub mod widgets;

pub use error::TuiError;
