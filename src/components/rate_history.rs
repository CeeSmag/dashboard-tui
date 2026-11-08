//! Fixed-capacity sample history for live graphs (`RateHistory`).
//!
//! Hosts push one value per tick and hand [`RateHistory::as_slice`] to a
//! [`ThroughputPanel`](crate::universal::ThroughputPanel) or waveform, with no ring-buffer
//! bookkeeping of their own.

/// Bounded, chronological (oldest first) list of recent samples.
///
/// # Examples
///
/// ```
/// use dashboard_tui::components::RateHistory;
///
/// let mut history = RateHistory::new(3);
/// for value in [1.0, 2.0, 3.0, 4.0] {
///     history.push(value);
/// }
/// assert_eq!(history.as_slice(), &[2.0, 3.0, 4.0]);
/// assert_eq!(history.latest(), 4.0);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RateHistory {
    samples: Vec<f64>,
    capacity: usize,
}

impl RateHistory {
    /// Default number of samples kept by [`RateHistory::default`]-style constructors.
    pub const DEFAULT_CAPACITY: usize = 120;

    /// Creates a history keeping at most `capacity` samples (minimum 1).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Appends `value`, dropping the oldest sample when full. Non-finite values count as `0`.
    pub fn push(&mut self, value: f64) {
        if self.capacity == 0 {
            self.capacity = Self::DEFAULT_CAPACITY;
        }
        if self.samples.len() >= self.capacity {
            self.samples.remove(0);
        }
        self.samples
            .push(if value.is_finite() { value } else { 0.0 });
    }

    /// Returns the samples, oldest first.
    #[must_use]
    #[inline]
    pub fn as_slice(&self) -> &[f64] {
        &self.samples
    }

    /// Returns the newest sample, or `0.0` when empty.
    #[must_use]
    pub fn latest(&self) -> f64 {
        self.samples.last().copied().unwrap_or(0.0)
    }

    /// Returns the number of stored samples.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns `true` if no samples are stored.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Removes all samples.
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}
