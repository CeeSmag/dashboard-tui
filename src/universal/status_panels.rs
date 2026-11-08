//! Universal telemetry section widgets: [`StatusPanel`] (labelled meter / text / rate rows)
//! and [`ThroughputPanel`] (live braille waveform with a rate header).
//!
//! Both are backend-agnostic [`SectionWidget`]s. The host supplies plain `fn(&B) -> …`
//! accessors, so drawing never allocates or copies the host's data.

use core::fmt::Write;

use crate::components::{DualWaveform, GradientMeter, SingleWaveform, format_rate, nice_ceil};
use crate::core::buffer::{Buffer, StackStr};
use crate::core::color::Color;
use crate::core::palette::ColorPalette;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::section::SectionWidget;

/// Width in columns reserved for the left-hand row label of a [`StatusPanel`].
const LABEL_COLUMNS: u16 = 9;

/// Value shown on a [`StatusPanel`] row.
///
/// # Examples
///
/// ```
/// use dashboard_tui::universal::StatusValue;
///
/// let value = StatusValue::Meter(42.0);
/// assert_eq!(value, StatusValue::Meter(42.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum StatusValue<'a> {
    /// A gradient meter filled to `0.0..=100.0` percent, followed by the percentage.
    Meter(f64),
    /// Plain text borrowed from the backend.
    Text(&'a str),
    /// Upload (`▲`) and download (`▼`) rates in bytes per second.
    Rates {
        /// Upload rate in bytes per second.
        up_bytes_per_sec: f64,
        /// Download rate in bytes per second.
        down_bytes_per_sec: f64,
    },
}

/// Accessor producing the current [`StatusValue`] of a row.
pub type StatusValueFn<B> = Box<dyn for<'a> Fn(&'a B) -> StatusValue<'a>>;

/// Writes optional detail text (e.g. `"(16.0G)"`) shown after a meter's percentage.
pub type StatusDetailFn<B> = Box<dyn Fn(&B, &mut StackStr<32>)>;

/// One labelled row of a [`StatusPanel`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::DashboardBackend;
/// use dashboard_tui::universal::{StatusRow, StatusValue};
///
/// struct App { cpu: f64 }
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let row = StatusRow::<App>::new("CPU", |app| StatusValue::Meter(app.cpu));
/// assert_eq!(row.label(), "CPU");
/// ```
pub struct StatusRow<B: DashboardBackend> {
    label: String,
    value: StatusValueFn<B>,
    detail: Option<StatusDetailFn<B>>,
}

impl<B: DashboardBackend> core::fmt::Debug for StatusRow<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StatusRow")
            .field("label", &self.label)
            .field("has_detail", &self.detail.is_some())
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> StatusRow<B> {
    /// Creates a row labelled `label` whose value is read through `value`.
    #[must_use]
    pub fn new<F>(label: impl Into<String>, value: F) -> Self
    where
        F: for<'a> Fn(&'a B) -> StatusValue<'a> + 'static,
    {
        Self {
            label: label.into(),
            value: Box::new(value),
            detail: None,
        }
    }

    /// A meter row filled to `percent(backend)` (`0.0..=100.0`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Color, PrioritySegment};
    /// use dashboard_tui::dashboard::DashboardBackend;
    /// use dashboard_tui::universal::StatusRow;
    ///
    /// struct App { cpu: f64 }
    /// impl DashboardBackend for App {
    ///     fn app_title(&self) -> &str { "App" }
    ///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
    ///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
    /// }
    ///
    /// let _row = StatusRow::<App>::meter("CPU", |app| app.cpu);
    /// ```
    #[must_use]
    pub fn meter(label: impl Into<String>, percent: impl Fn(&B) -> f64 + 'static) -> Self {
        Self::new(label, move |backend| StatusValue::Meter(percent(backend)))
    }

    /// A plain text row showing a string borrowed from the backend.
    #[must_use]
    pub fn text<F>(label: impl Into<String>, text: F) -> Self
    where
        F: for<'a> Fn(&'a B) -> &'a str + 'static,
    {
        Self::new(label, move |backend| StatusValue::Text(text(backend)))
    }

    /// An up/down rate row; `rates` returns `(upload, download)` in bytes per second.
    #[must_use]
    pub fn rates(label: impl Into<String>, rates: impl Fn(&B) -> (f64, f64) + 'static) -> Self {
        Self::new(label, move |backend| {
            let (up_bytes_per_sec, down_bytes_per_sec) = rates(backend);
            StatusValue::Rates {
                up_bytes_per_sec,
                down_bytes_per_sec,
            }
        })
    }

    /// Adds detail text written after a [`StatusValue::Meter`] percentage.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Fn(&B, &mut StackStr<32>) + 'static) -> Self {
        self.detail = Some(Box::new(detail));
        self
    }

    /// Returns the row label.
    #[must_use]
    #[inline]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Callback producing a section badge such as `("● ONLINE", Some(Color::Green))`.
pub type PanelBadgeFn<B> = fn(&B) -> (&'static str, Option<Color>);

/// Bordered panel of labelled rows: meters, text values and up/down rates.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::{DashboardBackend, Section};
/// use dashboard_tui::universal::StatusPanel;
///
/// struct App { cpu: f64 }
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let panel = StatusPanel::<App>::new("System")
///     .with_subtitle("app.service")
///     .with_badge(|_| ("● ONLINE", Some(Color::Green)))
///     .with_meter("CPU", |app| app.cpu)
///     .with_text("Daemon", |_| "running");
/// let _section = Section::display(Box::new(panel));
/// ```
pub struct StatusPanel<B: DashboardBackend> {
    title: String,
    subtitle: String,
    badge: Option<PanelBadgeFn<B>>,
    rows: Vec<StatusRow<B>>,
}

impl<B: DashboardBackend> core::fmt::Debug for StatusPanel<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StatusPanel")
            .field("title", &self.title)
            .field("rows_len", &self.rows.len())
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> StatusPanel<B> {
    /// Creates an empty panel titled `title`.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: String::new(),
            badge: None,
            rows: Vec::new(),
        }
    }

    /// Sets the text shown in the bottom border notch.
    #[must_use]
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Sets the live badge shown in the top border.
    #[must_use]
    pub fn with_badge(mut self, badge: PanelBadgeFn<B>) -> Self {
        self.badge = Some(badge);
        self
    }

    /// Appends a prepared row (for custom [`StatusValue`] logic or meter details).
    #[must_use]
    pub fn with_row(mut self, row: StatusRow<B>) -> Self {
        self.rows.push(row);
        self
    }

    /// Appends a meter row filled to `percent(backend)` (`0.0..=100.0`).
    #[must_use]
    pub fn with_meter(
        self,
        label: impl Into<String>,
        percent: impl Fn(&B) -> f64 + 'static,
    ) -> Self {
        self.with_row(StatusRow::meter(label, percent))
    }

    /// Appends a meter row followed by detail text such as `(16.0G)`.
    #[must_use]
    pub fn with_meter_detail(
        self,
        label: impl Into<String>,
        percent: impl Fn(&B) -> f64 + 'static,
        detail: impl Fn(&B, &mut StackStr<32>) + 'static,
    ) -> Self {
        self.with_row(StatusRow::meter(label, percent).with_detail(detail))
    }

    /// Appends a text row showing a string borrowed from the backend.
    #[must_use]
    pub fn with_text<F>(self, label: impl Into<String>, text: F) -> Self
    where
        F: for<'a> Fn(&'a B) -> &'a str + 'static,
    {
        self.with_row(StatusRow::text(label, text))
    }

    /// Appends an up/down rate row; `rates` returns `(upload, download)` in bytes per second.
    #[must_use]
    pub fn with_rates(
        self,
        label: impl Into<String>,
        rates: impl Fn(&B) -> (f64, f64) + 'static,
    ) -> Self {
        self.with_row(StatusRow::rates(label, rates))
    }
}

/// Writes `label` padded/clipped into the label column and returns the value column.
fn render_row_label(
    buffer: &mut Buffer,
    inner: Rect,
    row: crate::core::coordinates::RowIndex,
    label: &str,
    palette: &ColorPalette,
) -> crate::core::coordinates::ColumnIndex {
    let muted = Style::new().with_fg(palette.muted);
    let label_width = LABEL_COLUMNS.min(inner.width().get());
    let _ = buffer.set_string(inner.x(), row, label, muted, label_width);
    inner.x().saturating_add(label_width)
}

impl<B: DashboardBackend> SectionWidget<B> for StatusPanel<B> {
    fn title(&self, _backend: &B) -> String {
        self.title.clone()
    }

    fn subtitle(&self, _backend: &B) -> String {
        self.subtitle.clone()
    }

    fn badge(&self, backend: &B) -> (String, Option<Color>) {
        self.badge.map_or_else(
            || (String::new(), None),
            |badge| {
                let (text, color) = badge(backend);
                (text.to_string(), color)
            },
        )
    }

    fn min_height(&self) -> u16 {
        u16::try_from(self.rows.len().min(8))
            .unwrap_or(8)
            .saturating_add(2)
            .max(4)
    }

    fn preferred_height(&self) -> u16 {
        u16::try_from(self.rows.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2)
    }

    fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        if inner.is_empty() {
            return;
        }
        let text_style = Style::new().with_fg(palette.text);
        let right_edge = inner.right().get();

        for (row_offset, status_row) in self
            .rows
            .iter()
            .take(usize::from(inner.height().get()))
            .enumerate()
        {
            let row_index = u16::try_from(row_offset).unwrap_or(u16::MAX);
            let row =
                crate::core::coordinates::RowIndex::new(inner.y().get().saturating_add(row_index));
            let value_col = render_row_label(buffer, inner, row, status_row.label(), palette);
            let remaining = right_edge.saturating_sub(value_col.get());

            match (status_row.value)(backend) {
                StatusValue::Meter(percent) => {
                    let clamped = percent.clamp(0.0, 100.0);
                    let mut tail = StackStr::<48>::new();
                    let _ = write!(tail, "  {clamped:5.1}%");
                    if let Some(detail) = &status_row.detail {
                        let mut detail_text = StackStr::<32>::new();
                        detail(backend, &mut detail_text);
                        let _ = write!(tail, " {}", detail_text.as_str());
                    }
                    let tail_width = u16::try_from(tail.as_str().chars().count()).unwrap_or(0);
                    let meter_width = remaining.saturating_sub(tail_width).clamp(6, 30);
                    let after_meter = GradientMeter::new(clamped / 100.0, meter_width)
                        .with_tty_mode(tty_mode)
                        .render(buffer, value_col, row, palette);
                    let _ = buffer.set_string(
                        after_meter,
                        row,
                        tail.as_str(),
                        text_style,
                        right_edge.saturating_sub(after_meter.get()),
                    );
                }
                StatusValue::Text(text) => {
                    let _ = buffer.set_string(value_col, row, text, text_style, remaining);
                }
                StatusValue::Rates {
                    up_bytes_per_sec,
                    down_bytes_per_sec,
                } => {
                    let up_style = Style::new().with_fg(palette.ok);
                    let down_style = Style::new().with_fg(palette.info);
                    let mut col = buffer.set_string(value_col, row, "▲ ", up_style, remaining);
                    col = buffer.set_string(
                        col,
                        row,
                        format_rate(up_bytes_per_sec).as_str(),
                        text_style,
                        right_edge.saturating_sub(col.get()),
                    );
                    col = buffer.set_string(
                        col,
                        row,
                        "   ▼ ",
                        down_style,
                        right_edge.saturating_sub(col.get()),
                    );
                    let _ = buffer.set_string(
                        col,
                        row,
                        format_rate(down_bytes_per_sec).as_str(),
                        text_style,
                        right_edge.saturating_sub(col.get()),
                    );
                }
            }
        }
    }
}

/// Accessor returning a host-owned, chronological (oldest first) rate history in bytes/s.
pub type SeriesFn<B> = fn(&B) -> &[f64];

/// Bordered live traffic panel: a rate header plus a braille waveform.
///
/// With one series it draws a single waveform (e.g. network traffic); with two it draws
/// mirrored upload/download (or read/write) waveforms. The scale adapts to the visible peak.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::{DashboardBackend, Section};
/// use dashboard_tui::universal::ThroughputPanel;
///
/// struct App { up: Vec<f64>, down: Vec<f64> }
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let panel = ThroughputPanel::<App>::dual(
///     "Disk I/O", "Write", |app| app.up.as_slice(), "Read", |app| app.down.as_slice(),
/// )
/// .with_subtitle("physical disks");
/// let _section = Section::display(Box::new(panel));
/// ```
pub struct ThroughputPanel<B: DashboardBackend> {
    title: String,
    subtitle: String,
    top_label: String,
    top_series: SeriesFn<B>,
    bottom: Option<(String, SeriesFn<B>)>,
    min_scale_bytes_per_sec: f64,
    active_threshold_bytes_per_sec: f64,
}

impl<B: DashboardBackend> core::fmt::Debug for ThroughputPanel<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ThroughputPanel")
            .field("title", &self.title)
            .field("is_dual", &self.bottom.is_some())
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> ThroughputPanel<B> {
    /// Creates a single-series panel (e.g. total network traffic).
    #[must_use]
    pub fn single(title: impl Into<String>, label: impl Into<String>, series: SeriesFn<B>) -> Self {
        Self {
            title: title.into(),
            subtitle: String::new(),
            top_label: label.into(),
            top_series: series,
            bottom: None,
            min_scale_bytes_per_sec: 100.0 * 1024.0,
            active_threshold_bytes_per_sec: 1024.0,
        }
    }

    /// Creates a mirrored two-series panel (e.g. upload on top, download below).
    #[must_use]
    pub fn dual(
        title: impl Into<String>,
        top_label: impl Into<String>,
        top_series: SeriesFn<B>,
        bottom_label: impl Into<String>,
        bottom_series: SeriesFn<B>,
    ) -> Self {
        let mut panel = Self::single(title, top_label, top_series);
        panel.bottom = Some((bottom_label.into(), bottom_series));
        panel
    }

    /// Sets the text shown in the bottom border notch.
    #[must_use]
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Sets the smallest waveform scale in bytes per second (default 100 KB/s).
    #[must_use]
    pub const fn with_min_scale(mut self, bytes_per_sec: f64) -> Self {
        self.min_scale_bytes_per_sec = bytes_per_sec;
        self
    }

    /// Sets the combined latest rate above which the badge reads `● ACTIVE` (default 1 KB/s).
    #[must_use]
    pub const fn with_active_threshold(mut self, bytes_per_sec: f64) -> Self {
        self.active_threshold_bytes_per_sec = bytes_per_sec;
        self
    }

    fn latest_rates(&self, backend: &B) -> (f64, f64) {
        let top = (self.top_series)(backend).last().copied().unwrap_or(0.0);
        let bottom = self
            .bottom
            .as_ref()
            .and_then(|(_, series)| series(backend).last().copied())
            .unwrap_or(0.0);
        (top, bottom)
    }

    fn scale_ceiling(&self, backend: &B) -> f64 {
        let peak_of = |samples: &[f64]| samples.iter().copied().fold(0.0_f64, f64::max);
        let top_peak = peak_of((self.top_series)(backend));
        let bottom_peak = self
            .bottom
            .as_ref()
            .map_or(0.0, |(_, series)| peak_of(series(backend)));
        nice_ceil(
            top_peak.max(bottom_peak) * 1.25,
            self.min_scale_bytes_per_sec,
        )
    }
}

impl<B: DashboardBackend> SectionWidget<B> for ThroughputPanel<B> {
    fn title(&self, _backend: &B) -> String {
        self.title.clone()
    }

    fn subtitle(&self, _backend: &B) -> String {
        self.subtitle.clone()
    }

    fn badge(&self, backend: &B) -> (String, Option<Color>) {
        let (top, bottom) = self.latest_rates(backend);
        if top + bottom > self.active_threshold_bytes_per_sec {
            ("● ACTIVE".to_string(), Some(Color::Green))
        } else {
            ("● IDLE".to_string(), None)
        }
    }

    fn render(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        if inner.is_empty() {
            return;
        }
        let (top_rate, bottom_rate) = self.latest_rates(backend);
        let scale = self.scale_ceiling(backend);
        let top_samples = (self.top_series)(backend);

        let mut header = StackStr::<96>::new();
        let _ = write!(
            header,
            "{} {}",
            self.top_label,
            format_rate(top_rate).as_str()
        );
        if let Some((bottom_label, _)) = &self.bottom {
            let _ = write!(
                header,
                "   {} {}",
                bottom_label,
                format_rate(bottom_rate).as_str()
            );
        }
        let _ = buffer.set_string(
            inner.x(),
            inner.y(),
            header.as_str(),
            Style::new().with_fg(palette.text).bold(),
            inner.width().get(),
        );

        let graph_height = inner.height().get().saturating_sub(1);
        if graph_height == 0 {
            return;
        }
        let graph_area = Rect::new(
            inner.x(),
            inner.y().saturating_add(1),
            inner.width(),
            crate::core::rect::TerminalHeight::new(graph_height),
        );
        let mode = if tty_mode {
            crate::components::GraphSymbolMode::Tty
        } else {
            crate::components::GraphSymbolMode::Braille
        };

        if let Some((_, bottom_series)) = &self.bottom {
            DualWaveform::new(top_samples, bottom_series(backend), scale)
                .with_mode(mode)
                .render(buffer, graph_area, palette);
        } else {
            SingleWaveform::new(top_samples, scale)
                .with_mode(mode)
                .render(buffer, graph_area, palette.gradient_lut());
        }
    }
}
