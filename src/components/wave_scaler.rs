//! Adaptive auto-scaling state machine with hysteresis for telemetry waveforms.
//!
//! Directly ports the `btop` (`linux/btop_collect.cpp`) and `ref/dashboard/de_monitors.go`
//! (`WaveScaler`) algorithm:
//! - **Deadband (`10%..=100%`)**: If the visible peak sits between `down_ratio * scale`
//!   and `scale`, the current ceiling holds steady to prevent visual jitter.
//! - **Headroom (`1.30×`)**: When rescaling up, the new scale is set to `1.30×` of peak
//!   so spikes sit comfortably around ~77% height.
//! - **5-sample rolling average (`avg5 * 3.0`)**: When rescaling down after `down_limit`
//!   consecutive quiet updates, the scale decays smoothly toward `max(peak * 1.30, avg5 * 3.0)`
//!   rounded to 2 significant digits (`round_scale`).
//! - **40ms rapid-tick debounce**: Protects decay counters from high-frequency render polls.

use std::time::{Duration, Instant};

/// Configuration parameters for a [`WaveScaler`].
///
/// # Examples
///
/// ```
/// use tui_rs::components::WaveScalerConfig;
///
/// let config = WaveScalerConfig::new(256.0 * 1024.0);
/// assert_eq!(config.min_scale(), 262_144.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct WaveScalerConfig {
    min_scale: f64,
    headroom: f64,
    down_ratio: f64,
    down_limit: u32,
}

impl Default for WaveScalerConfig {
    #[inline]
    fn default() -> Self {
        Self {
            min_scale: 256.0 * 1024.0,
            headroom: 1.30,
            down_ratio: 0.10,
            down_limit: 5,
        }
    }
}

impl WaveScalerConfig {
    /// Creates a [`WaveScalerConfig`] with the given `min_scale` floor and `btop` defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// let cfg = WaveScalerConfig::new(1024.0);
    /// assert_eq!(cfg.min_scale(), 1024.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(min_scale: f64) -> Self {
        Self {
            min_scale: if min_scale > 0.0 {
                min_scale
            } else {
                256.0 * 1024.0
            },
            headroom: 1.30,
            down_ratio: 0.10,
            down_limit: 5,
        }
    }

    /// Sets the upward headroom multiplier (default `1.30`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// let cfg = WaveScalerConfig::new(1024.0).with_headroom(1.25);
    /// assert_eq!(cfg.headroom(), 1.25);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_headroom(mut self, headroom: f64) -> Self {
        if headroom > 1.0 {
            self.headroom = headroom;
        }
        self
    }

    /// Sets the lower deadband ratio (default `0.10`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// let cfg = WaveScalerConfig::new(1024.0).with_down_ratio(0.15);
    /// assert_eq!(cfg.down_ratio(), 0.15);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_down_ratio(mut self, ratio: f64) -> Self {
        if ratio > 0.0 && ratio < 1.0 {
            self.down_ratio = ratio;
        }
        self
    }

    /// Sets the number of consecutive low updates before decaying (default `5`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// let cfg = WaveScalerConfig::new(1024.0).with_down_limit(8);
    /// assert_eq!(cfg.down_limit(), 8);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_down_limit(mut self, limit: u32) -> Self {
        if limit > 0 {
            self.down_limit = limit;
        }
        self
    }

    /// Returns the configured minimum scale floor.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// assert_eq!(WaveScalerConfig::new(500.0).min_scale(), 500.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn min_scale(&self) -> f64 {
        self.min_scale
    }

    /// Returns the upward headroom multiplier.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// assert_eq!(WaveScalerConfig::default().headroom(), 1.30);
    /// ```
    #[must_use]
    #[inline]
    pub const fn headroom(&self) -> f64 {
        self.headroom
    }

    /// Returns the lower deadband ratio.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// assert_eq!(WaveScalerConfig::default().down_ratio(), 0.10);
    /// ```
    #[must_use]
    #[inline]
    pub const fn down_ratio(&self) -> f64 {
        self.down_ratio
    }

    /// Returns the consecutive low-update count required before down-scaling.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScalerConfig;
    ///
    /// assert_eq!(WaveScalerConfig::default().down_limit(), 5);
    /// ```
    #[must_use]
    #[inline]
    pub const fn down_limit(&self) -> u32 {
        self.down_limit
    }
}

/// Stateful auto-scaler with hysteresis for single or dual telemetry waveforms.
///
/// # Examples
///
/// ```
/// use tui_rs::components::{WaveScaler, WaveScalerConfig};
///
/// let mut scaler = WaveScaler::new(WaveScalerConfig::new(1000.0));
/// let scale = scaler.update(&[500.0, 2000.0, 1500.0]);
/// assert!(scale >= 2000.0);
/// ```
#[derive(Debug, Clone)]
pub struct WaveScaler {
    config: WaveScalerConfig,
    scale: f64,
    down_count: u32,
    last_step: Option<Instant>,
}

impl Default for WaveScaler {
    #[inline]
    fn default() -> Self {
        Self::new(WaveScalerConfig::default())
    }
}

impl WaveScaler {
    /// Creates a new [`WaveScaler`] with the specified configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{WaveScaler, WaveScalerConfig};
    ///
    /// let scaler = WaveScaler::new(WaveScalerConfig::new(1024.0));
    /// assert_eq!(scaler.current_scale(), 1024.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(config: WaveScalerConfig) -> Self {
        Self {
            scale: config.min_scale,
            config,
            down_count: 0,
            last_step: None,
        }
    }

    /// Returns the current active scale ceiling without mutating state.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::WaveScaler;
    ///
    /// let scaler = WaveScaler::default();
    /// assert!(scaler.current_scale() > 0.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn current_scale(&self) -> f64 {
        self.scale
    }

    /// Resets the scaler back to its minimum floor and clears hysteresis counters.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{WaveScaler, WaveScalerConfig};
    ///
    /// let mut scaler = WaveScaler::new(WaveScalerConfig::new(1000.0));
    /// let _ = scaler.update(&[50_000.0]);
    /// scaler.reset();
    /// assert_eq!(scaler.current_scale(), 1000.0);
    /// ```
    #[inline]
    pub fn reset(&mut self) {
        self.scale = self.config.min_scale;
        self.down_count = 0;
        self.last_step = None;
    }

    /// Updates and returns the scale ceiling for a single series of visible samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{WaveScaler, WaveScalerConfig};
    ///
    /// let mut scaler = WaveScaler::new(WaveScalerConfig::new(100.0));
    /// let ceiling = scaler.update(&[10.0, 500.0, 200.0]);
    /// assert!(ceiling >= 500.0);
    /// ```
    pub fn update(&mut self, samples: &[f64]) -> f64 {
        let mut peak = 0.0_f64;
        for &val in samples {
            if val > peak {
                peak = val;
            }
        }
        let n = samples.len().min(5);
        let avg5 = if n > 0 {
            let tail = &samples[samples.len().saturating_sub(n)..];
            let sum: f64 = tail.iter().copied().sum();
            #[expect(clippy::cast_precision_loss, reason = "n is at most 5")]
            {
                sum / (n as f64)
            }
        } else {
            0.0
        };
        self.apply_peak(peak, avg5)
    }

    /// Updates and returns a shared scale ceiling across both top and bottom
    /// series of a dual mirrored waveform (`update_sync`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{WaveScaler, WaveScalerConfig};
    ///
    /// let mut scaler = WaveScaler::new(WaveScalerConfig::new(100.0));
    /// let ceiling = scaler.update_sync(&[200.0], &[800.0]);
    /// assert!(ceiling >= 800.0);
    /// ```
    pub fn update_sync(&mut self, top: &[f64], bottom: &[f64]) -> f64 {
        let mut peak = 0.0_f64;
        for &val in top.iter().chain(bottom.iter()) {
            if val > peak {
                peak = val;
            }
        }
        let mut avg5 = 0.0_f64;
        for s in [top, bottom] {
            let n = s.len().min(5);
            if n > 0 {
                let sum: f64 = s[s.len().saturating_sub(n)..].iter().copied().sum();
                #[expect(clippy::cast_precision_loss, reason = "n is at most 5")]
                let a = sum / (n as f64);
                if a > avg5 {
                    avg5 = a;
                }
            }
        }
        self.apply_peak(peak, avg5)
    }

    fn apply_peak(&mut self, peak: f64, avg5: f64) -> f64 {
        if self.scale < self.config.min_scale {
            self.scale = self.config.min_scale;
        }
        let now = Instant::now();
        let rapid_tick = self
            .last_step
            .is_some_and(|prev| now.duration_since(prev) < Duration::from_millis(40));

        // 1. Scale UP immediately if peak exceeds current ceiling
        if peak > self.scale {
            self.scale = round_scale(peak * self.config.headroom, self.config.min_scale);
            self.down_count = 0;
            self.last_step = Some(now);
            return self.scale;
        }

        // 2. Deadband: peak is between down_ratio*scale and scale -> hold steady
        let low_threshold = self.scale * self.config.down_ratio;
        if peak >= low_threshold || self.scale <= self.config.min_scale {
            if !rapid_tick {
                self.down_count = 0;
            }
            return self.scale;
        }

        // 3. Peak is below deadband -> count consecutive quiet updates
        if !rapid_tick {
            self.down_count = self.down_count.saturating_add(1);
            self.last_step = Some(now);
        }
        if self.down_count >= self.config.down_limit {
            self.down_count = 0;
            let mut target = peak * self.config.headroom;
            let avg_floor = avg5 * 3.0;
            if avg_floor > target {
                target = avg_floor;
            }
            if target < self.config.min_scale {
                target = self.config.min_scale;
            }
            self.scale = round_scale(target, self.config.min_scale);
        }
        self.scale
    }
}

/// Rounds a scale value up to 2 significant digits so axis labels stay clean
/// without jumping by 2×–5× steps.
///
/// # Examples
///
/// ```
/// use tui_rs::components::round_scale;
///
/// assert_eq!(round_scale(100.0, 500.0), 500.0);
/// assert!(round_scale(1234.0, 100.0) >= 1234.0);
/// ```
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "exponent is within normal f64 range"
)]
pub fn round_scale(val: f64, floor: f64) -> f64 {
    if val <= floor {
        return floor;
    }
    let exp = val.log10().floor() as i32;
    let step = 10.0_f64.powi(exp.saturating_sub(1));
    if step <= 0.0 || !step.is_finite() {
        return val;
    }
    let rounded = (val / step).ceil() * step;
    if rounded < floor {
        floor
    } else {
        rounded
    }
}

/// Rounds `val` up to a human-friendly 1–2–5 decade ceiling (`1, 2, 5, 10 × 10^n`).
///
/// # Examples
///
/// ```
/// use tui_rs::components::nice_ceil;
///
/// assert_eq!(nice_ceil(150.0, 100.0), 200.0);
/// assert_eq!(nice_ceil(350.0, 100.0), 500.0);
/// ```
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "exponent is within normal f64 range"
)]
pub fn nice_ceil(val: f64, floor: f64) -> f64 {
    if val <= floor {
        return floor;
    }
    let exp = val.log10().floor();
    let base = 10.0_f64.powi(exp as i32);
    if base <= 0.0 || !base.is_finite() {
        return val;
    }
    let frac = val / base;
    let nice = if frac <= 1.0 {
        1.0
    } else if frac <= 2.0 {
        2.0
    } else if frac <= 5.0 {
        5.0
    } else {
        10.0
    };
    let res = nice * base;
    if res < floor {
        floor
    } else {
        res
    }
}
