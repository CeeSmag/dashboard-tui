//! High-resolution telemetry and data-visualization components.
//!
//! Includes:
//! - [`GraphSymbolMode`], [`braille_up_cell`], [`braille_down_cell`], [`graph_up_cell`],
//!   [`graph_down_cell`]: 5×5 sub-cell graph lookup tables (`Braille`, `Block`, `Tty`).
//! - [`WaveScaler`], [`WaveScalerConfig`], [`round_scale`], [`nice_ceil`]: Adaptive
//!   auto-scaling state machine with deadband hysteresis and 5-sample rolling average decay.
//! - [`TimeSeries`], [`TimedSample`], [`resample_slice_into`]: Bounded ring buffer and
//!   peak-preserving bucket resampler.
//! - [`SingleWaveform`] and [`DualWaveform`]: Zero-allocation upward and mirrored
//!   upward/downward waveform visualizers.
//! - [`GradientMeter`], [`SliderBar`], [`HalfBlockQr`]: Progress meters, interactive
//!   sliders, and half-block QR matrix renderers.
//! - [`format_bytes`], [`format_rate`], [`format_rate_short`], [`spinner_frame`]:
//!   Stack-allocated byte/throughput formatters and Braille spinner frames.

pub mod dual_waveform;
pub mod formatters;
pub mod graph_symbols;
pub mod meter;
pub mod qr_code;
pub mod qr_widget;
pub mod single_waveform;
pub mod slider;
pub mod time_series;
pub mod wave_scaler;

pub use dual_waveform::DualWaveform;
pub use formatters::{format_bytes, format_rate, format_rate_short, spinner_frame};
pub use graph_symbols::{
    GraphSymbolMode, braille_down_cell, braille_up_cell, clamp_dots, graph_down_cell,
    graph_up_cell, row_wave_dots, total_wave_dots,
};
pub use meter::GradientMeter;
pub use qr_code::{HalfBlockQr, QrCodeMatrix, QrErrorCorrection};
pub use qr_widget::QrWidget;
pub use single_waveform::SingleWaveform;
pub use slider::SliderBar;
pub use time_series::{TimeSeries, TimedSample, resample_slice_into, resample_slice_scaled_into};
pub use wave_scaler::{WaveScaler, WaveScalerConfig, nice_ceil, round_scale};
