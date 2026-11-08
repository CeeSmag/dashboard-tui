//! Line and scatter chart with labelled axes (`Chart`, `Dataset`, `Axis`).

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
use crate::core::grapheme::display_width;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::widget::Widget;
use crate::widgets::plot::{DotPlotter, Marker, round_to_i64, scale_to_dots};

/// How a [`Dataset`] is drawn.
///
/// # Examples
///
/// ```
/// use dashboard_tui::widgets::GraphType;
/// assert_eq!(GraphType::default(), GraphType::Scatter);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum GraphType {
    /// Individual points.
    #[default]
    Scatter,
    /// Points joined by straight lines.
    Line,
}

/// One series of `(x, y)` points.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::Style;
/// use dashboard_tui::widgets::{Dataset, GraphType};
///
/// let data = [(0.0, 1.0), (1.0, 3.0)];
/// let dataset = Dataset::new("cpu", &data).graph_type(GraphType::Line).style(Style::new().bold());
/// assert_eq!(dataset.name(), "cpu");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset<'a> {
    name: &'a str,
    data: &'a [(f64, f64)],
    style: Style,
    graph_type: GraphType,
    marker: Marker,
}

impl<'a> Dataset<'a> {
    /// Creates a dataset named `name` over `data`.
    #[must_use]
    pub const fn new(name: &'a str, data: &'a [(f64, f64)]) -> Self {
        Self {
            name,
            data,
            style: Style::DEFAULT,
            graph_type: GraphType::Scatter,
            marker: Marker::Braille,
        }
    }

    /// Sets the drawing style (foreground color).
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets scatter or line drawing.
    #[must_use]
    pub const fn graph_type(mut self, graph_type: GraphType) -> Self {
        self.graph_type = graph_type;
        self
    }

    /// Sets the dot glyph family (default braille).
    #[must_use]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// Returns the dataset name.
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
    }
}

/// One axis: value bounds, evenly spaced labels and an optional title.
///
/// # Examples
///
/// ```
/// use dashboard_tui::widgets::Axis;
///
/// let axis = Axis::new([0.0, 60.0]).labels(["0s", "30s", "60s"]).title("time");
/// assert_eq!(axis.bounds(), [0.0, 60.0]);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Axis<'a> {
    title: Option<&'a str>,
    bounds: [f64; 2],
    labels: Vec<&'a str>,
    style: Style,
}

impl<'a> Axis<'a> {
    /// Creates an axis spanning `bounds` (`[min, max]`).
    #[must_use]
    pub const fn new(bounds: [f64; 2]) -> Self {
        Self {
            title: None,
            bounds,
            labels: Vec::new(),
            style: Style::DEFAULT,
        }
    }

    /// Sets evenly spaced labels from the axis start to its end.
    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = &'a str>) -> Self {
        self.labels = labels.into_iter().collect();
        self
    }

    /// Sets a title (drawn at the axis end).
    #[must_use]
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Sets the style of the axis line and labels.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns the axis bounds.
    #[must_use]
    pub const fn bounds(&self) -> [f64; 2] {
        self.bounds
    }
}

/// A chart of one or more [`Dataset`]s over an x and a y [`Axis`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Rect, Style, Widget};
/// use dashboard_tui::widgets::{Axis, Chart, Dataset, GraphType};
///
/// let data = [(0.0, 0.0), (5.0, 10.0), (10.0, 5.0)];
/// let area = Rect::from_xywh(0, 0, 30, 8);
/// let mut buffer = Buffer::empty(area);
/// Chart::new([Dataset::new("load", &data).graph_type(GraphType::Line)])
///     .x_axis(Axis::new([0.0, 10.0]).labels(["0", "5", "10"]))
///     .y_axis(Axis::new([0.0, 10.0]).labels(["0", "10"]))
///     .render(area, &mut buffer);
/// let lines = buffer.to_visual_lines();
/// assert!(lines[7].contains('5') && lines[7].contains("10"));
/// assert!(lines.iter().any(|row| row.contains('│')));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chart<'a> {
    datasets: Vec<Dataset<'a>>,
    x_axis: Axis<'a>,
    y_axis: Axis<'a>,
    style: Style,
    show_legend: bool,
}

impl<'a> Chart<'a> {
    /// Creates a chart from datasets with default `0..1` axes.
    #[must_use]
    pub fn new(datasets: impl IntoIterator<Item = Dataset<'a>>) -> Self {
        Self {
            datasets: datasets.into_iter().collect(),
            x_axis: Axis::new([0.0, 1.0]),
            y_axis: Axis::new([0.0, 1.0]),
            style: Style::DEFAULT,
            show_legend: true,
        }
    }

    /// Sets the horizontal axis.
    #[must_use]
    pub fn x_axis(mut self, axis: Axis<'a>) -> Self {
        self.x_axis = axis;
        self
    }

    /// Sets the vertical axis.
    #[must_use]
    pub fn y_axis(mut self, axis: Axis<'a>) -> Self {
        self.y_axis = axis;
        self
    }

    /// Sets the style behind the whole chart.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Shows or hides the dataset-name legend in the top-right corner (default shown).
    #[must_use]
    pub const fn legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }
}

/// Cell geometry of a chart: where the axes and the plotting area sit inside the widget area.
#[derive(Debug, Clone, Copy)]
struct PlotGeometry {
    area: Rect,
    y_label_width: u16,
    axis_column: u16,
    plot_left: u16,
    plot_top: u16,
    plot_width: u16,
    plot_height: u16,
    bottom_row: u16,
}

impl PlotGeometry {
    fn plot_area(&self) -> Rect {
        Rect::new(
            ColumnIndex::new(self.plot_left),
            RowIndex::new(self.plot_top),
            TerminalWidth::new(self.plot_width),
            TerminalHeight::new(self.plot_height),
        )
    }
}

impl Chart<'_> {
    fn geometry(&self, area: Rect) -> Option<PlotGeometry> {
        let y_label_width = self
            .y_axis
            .labels
            .iter()
            .map(|label| display_width(label).as_u16())
            .max()
            .unwrap_or(0);
        let axis_column = area.x().get().saturating_add(y_label_width);
        let title_rows = u16::from(self.y_axis.title.is_some());
        let label_rows = u16::from(!self.x_axis.labels.is_empty() || self.x_axis.title.is_some());
        let plot_left = axis_column.saturating_add(1);
        let plot_top = area.y().get().saturating_add(title_rows);
        let plot_width = area.right().get().saturating_sub(plot_left);
        let plot_height = area
            .height()
            .get()
            .saturating_sub(title_rows)
            .saturating_sub(1)
            .saturating_sub(label_rows);
        if plot_width == 0 || plot_height == 0 {
            return None;
        }
        Some(PlotGeometry {
            area,
            y_label_width,
            axis_column,
            plot_left,
            plot_top,
            plot_width,
            plot_height,
            bottom_row: plot_top.saturating_add(plot_height),
        })
    }

    fn render_axis_lines(&self, buffer: &mut Buffer, geometry: &PlotGeometry) {
        for row in geometry.plot_top..geometry.bottom_row {
            let _ = buffer.set_string(
                ColumnIndex::new(geometry.axis_column),
                RowIndex::new(row),
                "│",
                self.y_axis.style,
                1,
            );
        }
        let bottom = RowIndex::new(geometry.bottom_row);
        let _ = buffer.set_string(
            ColumnIndex::new(geometry.axis_column),
            bottom,
            "└",
            self.x_axis.style,
            1,
        );
        let plot_right = geometry.plot_left.saturating_add(geometry.plot_width);
        for column in geometry.plot_left..plot_right {
            let _ = buffer.set_string(ColumnIndex::new(column), bottom, "─", self.x_axis.style, 1);
        }
    }

    fn render_y_axis_text(&self, buffer: &mut Buffer, geometry: &PlotGeometry) {
        let count = self.y_axis.labels.len();
        for (index, label) in self.y_axis.labels.iter().enumerate() {
            let from_bottom = if count > 1 {
                usize::from(geometry.plot_height.saturating_sub(1)) * index / (count - 1)
            } else {
                0
            };
            let row = geometry
                .plot_top
                .saturating_add(geometry.plot_height.saturating_sub(1))
                .saturating_sub(u16::try_from(from_bottom).unwrap_or(0));
            let label_width = display_width(label).as_u16().min(geometry.y_label_width);
            let _ = buffer.set_string(
                ColumnIndex::new(
                    geometry
                        .area
                        .x()
                        .get()
                        .saturating_add(geometry.y_label_width - label_width),
                ),
                RowIndex::new(row),
                label,
                self.y_axis.style,
                label_width,
            );
        }
        if let Some(title) = self.y_axis.title {
            let _ = buffer.set_string(
                geometry.area.x(),
                geometry.area.y(),
                title,
                self.y_axis.style,
                geometry.area.width().get(),
            );
        }
    }

    fn render_x_axis_text(&self, buffer: &mut Buffer, geometry: &PlotGeometry) {
        let label_row = RowIndex::new(geometry.bottom_row.saturating_add(1));
        let right_edge = geometry.area.right().get();
        let count = self.x_axis.labels.len();
        for (index, label) in self.x_axis.labels.iter().enumerate() {
            let label_width = display_width(label).as_u16();
            let anchor = if count > 1 {
                usize::from(geometry.plot_width.saturating_sub(1)) * index / (count - 1)
            } else {
                0
            };
            let anchor = geometry
                .plot_left
                .saturating_add(u16::try_from(anchor).unwrap_or(0));
            // First label left-aligned, last right-aligned, middle ones centered.
            let start = if index == 0 {
                geometry.plot_left
            } else if index + 1 == count {
                anchor.saturating_sub(label_width.saturating_sub(1))
            } else {
                anchor.saturating_sub(label_width / 2)
            };
            let start = start.max(geometry.area.x().get());
            let _ = buffer.set_string(
                ColumnIndex::new(start),
                label_row,
                label,
                self.x_axis.style,
                right_edge.saturating_sub(start),
            );
        }
        if let Some(title) = self.x_axis.title {
            let title_width = display_width(title).as_u16();
            let _ = buffer.set_string(
                ColumnIndex::new(right_edge.saturating_sub(title_width)),
                label_row,
                title,
                self.x_axis.style,
                title_width,
            );
        }
    }

    fn render_datasets(&self, buffer: &mut Buffer, geometry: &PlotGeometry) {
        for dataset in &self.datasets {
            let plotter = DotPlotter::new(geometry.plot_area(), dataset.marker);
            let span_x = f64::from(plotter.dots_wide().saturating_sub(1));
            let span_y = f64::from(plotter.dots_high().saturating_sub(1));
            let mut previous: Option<(f64, f64)> = None;
            for &(x, y) in dataset.data {
                let point = scale_to_dots(x, self.x_axis.bounds, span_x, false).zip(scale_to_dots(
                    y,
                    self.y_axis.bounds,
                    span_y,
                    true,
                ));
                match (dataset.graph_type, point, previous) {
                    (GraphType::Line, Some(current), Some(last)) => {
                        plotter.line(buffer, last, current, dataset.style);
                    }
                    (GraphType::Line, Some(current), None)
                    | (GraphType::Scatter, Some(current), _) => plotter.set_dot(
                        buffer,
                        round_to_i64(current.0),
                        round_to_i64(current.1),
                        dataset.style,
                    ),
                    (_, None, _) => {}
                }
                previous = point;
            }
        }
    }

    fn render_legend(&self, buffer: &mut Buffer, geometry: &PlotGeometry) {
        let mut row = geometry.plot_top;
        for dataset in &self.datasets {
            if dataset.name.is_empty() || row >= geometry.bottom_row {
                continue;
            }
            let name_width = display_width(dataset.name).as_u16();
            let start = geometry
                .area
                .right()
                .get()
                .saturating_sub(name_width.saturating_add(2));
            if start <= geometry.plot_left {
                continue;
            }
            let _ = buffer.set_string(
                ColumnIndex::new(start),
                RowIndex::new(row),
                "● ",
                dataset.style,
                2,
            );
            let _ = buffer.set_string(
                ColumnIndex::new(start.saturating_add(2)),
                RowIndex::new(row),
                dataset.name,
                self.style,
                name_width,
            );
            row = row.saturating_add(1);
        }
    }
}

impl Widget for Chart<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.width().get() < 3 || area.height().get() < 3 {
            return;
        }
        buffer.set_style(area, self.style);
        let Some(geometry) = self.geometry(area) else {
            return;
        };
        self.render_axis_lines(buffer, &geometry);
        self.render_y_axis_text(buffer, &geometry);
        self.render_x_axis_text(buffer, &geometry);
        self.render_datasets(buffer, &geometry);
        if self.show_legend {
            self.render_legend(buffer, &geometry);
        }
    }
}
