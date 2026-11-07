//! Fixed-capacity telemetry ring buffer and time-window peak bucket resampler.
//!
//! Ports `TimeSeries` and `btResample` from `ref/dashboard/de_monitors.go`.
//! Preserves spikes across arbitrary terminal widths by taking the maximum
//! sample within each bucket and holding recent readings across short gaps
//! (up to 3 seconds) so real-time waveforms scroll smoothly without dropouts.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A single timestamped telemetry sample.
///
/// # Examples
///
/// ```
/// use std::time::Instant;
/// use tui_rs::components::TimedSample;
///
/// let sample = TimedSample::new(Instant::now(), 1024.0);
/// assert_eq!(sample.value(), 1024.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedSample {
    timestamp: Instant,
    value: f64,
}

impl TimedSample {
    /// Creates a new [`TimedSample`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::components::TimedSample;
    ///
    /// let s = TimedSample::new(Instant::now(), 42.5);
    /// assert_eq!(s.value(), 42.5);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(timestamp: Instant, value: f64) -> Self {
        Self { timestamp, value }
    }

    /// Returns the sample timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::components::TimedSample;
    ///
    /// let now = Instant::now();
    /// let s = TimedSample::new(now, 1.0);
    /// assert_eq!(s.timestamp(), now);
    /// ```
    #[must_use]
    #[inline]
    pub const fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Returns the sample value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::components::TimedSample;
    ///
    /// let s = TimedSample::new(Instant::now(), 99.0);
    /// assert_eq!(s.value(), 99.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn value(&self) -> f64 {
        self.value
    }
}

/// Bounded ring buffer of timestamped telemetry samples.
///
/// # Examples
///
/// ```
/// use std::time::{Duration, Instant};
/// use tui_rs::components::TimeSeries;
///
/// let mut ts = TimeSeries::with_capacity(120);
/// let now = Instant::now();
/// ts.push_at(now, 512.0);
/// assert_eq!(ts.len(), 1);
/// assert_eq!(ts.latest_value(), 512.0);
/// ```
#[derive(Debug, Clone)]
pub struct TimeSeries {
    samples: VecDeque<TimedSample>,
    capacity: usize,
}

impl Default for TimeSeries {
    #[inline]
    fn default() -> Self {
        Self::with_capacity(600)
    }
}

impl TimeSeries {
    /// Creates a new [`TimeSeries`] with the given maximum sample capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// let ts = TimeSeries::with_capacity(300);
    /// assert!(ts.is_empty());
    /// assert_eq!(ts.capacity(), 300);
    /// ```
    #[must_use]
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            samples: VecDeque::with_capacity(cap),
            capacity: cap,
        }
    }

    /// Pushes a sample stamped with `Instant::now()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// let mut ts = TimeSeries::with_capacity(10);
    /// ts.push(100.0);
    /// assert_eq!(ts.len(), 1);
    /// ```
    #[inline]
    pub fn push(&mut self, value: f64) {
        self.push_at(Instant::now(), value);
    }

    /// Pushes a sample with an explicit timestamp, evicting the oldest sample if full.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use tui_rs::components::TimeSeries;
    ///
    /// let mut ts = TimeSeries::with_capacity(2);
    /// let now = Instant::now();
    /// ts.push_at(now, 1.0);
    /// ts.push_at(now, 2.0);
    /// ts.push_at(now, 3.0);
    /// assert_eq!(ts.len(), 2);
    /// assert_eq!(ts.latest_value(), 3.0);
    /// ```
    #[inline]
    pub fn push_at(&mut self, timestamp: Instant, value: f64) {
        if self.samples.len() >= self.capacity {
            let _ = self.samples.pop_front();
        }
        self.samples.push_back(TimedSample::new(timestamp, value));
    }

    /// Clears all stored samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// let mut ts = TimeSeries::with_capacity(10);
    /// ts.push(10.0);
    /// ts.clear();
    /// assert!(ts.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Returns the number of stored samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// let ts = TimeSeries::with_capacity(10);
    /// assert_eq!(ts.len(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns `true` if no samples are stored.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// assert!(TimeSeries::default().is_empty());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns the maximum capacity of the ring buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// assert_eq!(TimeSeries::with_capacity(64).capacity(), 64);
    /// ```
    #[must_use]
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the most recently pushed value, or `0.0` if empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::TimeSeries;
    ///
    /// let mut ts = TimeSeries::default();
    /// assert_eq!(ts.latest_value(), 0.0);
    /// ts.push(42.0);
    /// assert_eq!(ts.latest_value(), 42.0);
    /// ```
    #[must_use]
    #[inline]
    pub fn latest_value(&self) -> f64 {
        self.samples.back().map_or(0.0, TimedSample::value)
    }

    /// Resamples the time series into `buckets.len()` equal sub-intervals across
    /// `[now - window, now]`, storing the peak value per bucket and holding recent
    /// values across short gaps (`<= 3s`).
    ///
    /// Writing into a caller-provided slice keeps the render hot-path allocation-free.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// use tui_rs::components::TimeSeries;
    ///
    /// let mut ts = TimeSeries::with_capacity(16);
    /// let now = Instant::now();
    /// ts.push_at(now, 100.0);
    /// let mut out = [0.0_f64; 8];
    /// ts.resample_into(now, Duration::from_secs(10), &mut out);
    /// assert_eq!(out[7], 100.0);
    /// ```
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "bucket index is clamped to 0..buckets.len()"
    )]
    pub fn resample_into(&self, now: Instant, window: Duration, buckets: &mut [f64]) {
        buckets.fill(0.0);
        if buckets.is_empty() || self.samples.is_empty() || window.is_zero() {
            return;
        }
        let num_buckets = buckets.len();
        let window_secs = window.as_secs_f64();
        let step_secs = window_secs / (num_buckets as f64);
        if step_secs <= 0.0 {
            return;
        }

        // Track which buckets received at least one direct sample.
        // For typical terminal widths (<= 512 buckets), we can track seen state
        // by initializing unseen buckets to -1.0 internally and clamping to 0.0 after.
        buckets.fill(-1.0);

        let mut prev_val = 0.0_f64;
        let mut prev_age_secs = f64::INFINITY;

        for sample in &self.samples {
            let age_secs = now
                .saturating_duration_since(sample.timestamp)
                .as_secs_f64();
            if age_secs > window_secs {
                // Sample precedes window; remember as carry-over candidate
                prev_val = sample.value;
                prev_age_secs = age_secs;
                continue;
            }
            let elapsed_from_start = (window_secs - age_secs).max(0.0);
            let idx =
                ((elapsed_from_start / step_secs) as usize).min(num_buckets.saturating_sub(1));
            if sample.value > buckets[idx] {
                buckets[idx] = sample.value;
            }
        }

        let max_hold_buckets = ((3.0 / step_secs).ceil() as usize).max(2);
        let mut last_val = prev_val;
        let mut hold_count = if prev_age_secs <= window_secs + 3.0 {
            0
        } else {
            max_hold_buckets
        };

        for slot in buckets.iter_mut() {
            if *slot >= 0.0 {
                last_val = *slot;
                hold_count = 0;
            } else {
                hold_count = hold_count.saturating_add(1);
                if hold_count <= max_hold_buckets {
                    *slot = last_val;
                } else {
                    *slot = 0.0;
                }
            }
        }
    }
}

/// Resamples a plain slice of untimed samples into `out.len()` buckets by
/// right-aligning recent samples (newest at the right edge) or downsampling
/// via peak-per-bucket when `samples.len() > out.len()`.
///
/// # Examples
///
/// ```
/// use tui_rs::components::resample_slice_into;
///
/// let samples = [10.0, 20.0, 30.0];
/// let mut out = [0.0; 5];
/// resample_slice_into(&samples, &mut out);
/// assert_eq!(out, [0.0, 0.0, 10.0, 20.0, 30.0]);
/// ```
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "indices bounded by slice lengths"
)]
pub fn resample_slice_into(samples: &[f64], out: &mut [f64]) {
    out.fill(0.0);
    if samples.is_empty() || out.is_empty() {
        return;
    }
    let out_len = out.len();
    let in_len = samples.len();
    if in_len <= out_len {
        let offset = out_len.saturating_sub(in_len);
        for (i, &val) in samples.iter().enumerate() {
            out[offset + i] = val.max(0.0);
        }
    } else {
        for (bucket_idx, slot) in out.iter_mut().enumerate() {
            let start = ((bucket_idx as f64 / out_len as f64) * in_len as f64) as usize;
            let end = ((((bucket_idx + 1) as f64) / out_len as f64) * in_len as f64) as usize;
            let end = end.max(start + 1).min(in_len);
            let mut peak = 0.0_f64;
            for &v in &samples[start..end] {
                if v > peak {
                    peak = v;
                }
            }
            *slot = peak;
        }
    }
}

/// Resamples a slice of samples across the **entire** `out` slice (`0..out.len()`) so
/// waveforms always fill 100% of the available horizontal width and dynamically stretch
/// or compress when the terminal window is resized.
///
/// - When `samples.len() >= out.len()`, downsamples using peak-per-bucket aggregation.
/// - When `samples.len() < out.len()`, smoothly interpolates across all `out.len()`
///   buckets so no leading half of the graph is left blank on wide terminals.
///
/// # Examples
///
/// ```
/// use tui_rs::components::resample_slice_scaled_into;
///
/// let samples = [10.0, 20.0, 30.0];
/// let mut out = [0.0; 5];
/// resample_slice_scaled_into(&samples, &mut out);
/// assert!((out[0] - 10.0).abs() < 1e-6);
/// assert!((out[2] - 20.0).abs() < 1e-6);
/// assert!((out[4] - 30.0).abs() < 1e-6);
/// ```
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "indices bounded by slice lengths"
)]
pub fn resample_slice_scaled_into(samples: &[f64], out: &mut [f64]) {
    out.fill(0.0);
    if samples.is_empty() || out.is_empty() {
        return;
    }
    let out_len = out.len();
    let in_len = samples.len();
    if in_len == 1 {
        out.fill(samples[0].max(0.0));
        return;
    }
    if out_len == 1 {
        let mut peak = 0.0_f64;
        for &v in samples {
            if v > peak {
                peak = v;
            }
        }
        out[0] = peak;
        return;
    }
    if in_len >= out_len {
        for (bucket_idx, slot) in out.iter_mut().enumerate() {
            let start = ((bucket_idx as f64 / out_len as f64) * in_len as f64) as usize;
            let end = ((((bucket_idx + 1) as f64) / out_len as f64) * in_len as f64) as usize;
            let end = end.max(start + 1).min(in_len);
            let mut peak = 0.0_f64;
            for &v in &samples[start..end] {
                if v > peak {
                    peak = v;
                }
            }
            *slot = peak;
        }
    } else {
        let max_in = (in_len - 1) as f64;
        let max_out = (out_len - 1) as f64;
        for (bucket_idx, slot) in out.iter_mut().enumerate() {
            let pos = (bucket_idx as f64 / max_out) * max_in;
            let idx0 = (pos.floor() as usize).min(in_len - 1);
            let idx1 = (idx0 + 1).min(in_len - 1);
            let frac = pos - (idx0 as f64);
            let v0 = samples[idx0].max(0.0);
            let v1 = samples[idx1].max(0.0);
            *slot = v0 + (v1 - v0) * frac;
        }
    }
}
