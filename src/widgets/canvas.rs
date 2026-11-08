//! Free-form drawing surface (`Canvas`) with floating-point coordinates.
//!
//! Draw points, lines, rectangles and circles in your own coordinate system; the canvas maps
//! them onto sub-cell dots (braille by default), clipped to its area.

use crate::core::buffer::Buffer;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::widget::Widget;
use crate::widgets::plot::{DotPlotter, Marker, scale_to_dots};

/// A shape drawn on a [`Canvas`], in canvas coordinates.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::Style;
/// use dashboard_tui::widgets::Shape;
///
/// let line = Shape::Line { from: (0.0, 0.0), to: (10.0, 5.0), style: Style::DEFAULT };
/// assert!(matches!(line, Shape::Line { .. }));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Shape<'a> {
    /// Individual points.
    Points {
        /// Coordinates `(x, y)`.
        coords: &'a [(f64, f64)],
        /// Style (foreground color) of the points.
        style: Style,
    },
    /// A straight line.
    Line {
        /// Start `(x, y)`.
        from: (f64, f64),
        /// End `(x, y)`.
        to: (f64, f64),
        /// Line style.
        style: Style,
    },
    /// An axis-aligned rectangle outline.
    Rectangle {
        /// Left edge.
        x: f64,
        /// Bottom edge.
        y: f64,
        /// Width in canvas units.
        width: f64,
        /// Height in canvas units.
        height: f64,
        /// Outline style.
        style: Style,
    },
    /// A circle outline in canvas units (it looks elliptical if the axes use different scales).
    Circle {
        /// Center x.
        x: f64,
        /// Center y.
        y: f64,
        /// Radius in canvas units of the x axis.
        radius: f64,
        /// Outline style.
        style: Style,
    },
}

/// A coordinate-space drawing surface.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Rect, Style, Widget};
/// use dashboard_tui::widgets::{Canvas, Shape};
///
/// let area = Rect::from_xywh(0, 0, 10, 5);
/// let mut buffer = Buffer::empty(area);
/// Canvas::new([0.0, 100.0], [0.0, 100.0])
///     .shape(Shape::Line { from: (0.0, 0.0), to: (100.0, 100.0), style: Style::DEFAULT })
///     .shape(Shape::Circle { x: 50.0, y: 50.0, radius: 20.0, style: Style::DEFAULT })
///     .render(area, &mut buffer);
/// // The diagonal touches the bottom-left and top-right cells.
/// let lines = buffer.to_visual_lines();
/// assert_ne!(lines[4].chars().next(), Some(' '));
/// assert_ne!(lines[0].chars().nth(9), Some(' '));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Canvas<'a> {
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    marker: Marker,
    shapes: Vec<Shape<'a>>,
    style: Style,
}

impl<'a> Canvas<'a> {
    /// Creates a canvas covering `x_bounds` × `y_bounds` (`[min, max]` each).
    #[must_use]
    pub const fn new(x_bounds: [f64; 2], y_bounds: [f64; 2]) -> Self {
        Self {
            x_bounds,
            y_bounds,
            marker: Marker::Braille,
            shapes: Vec::new(),
            style: Style::DEFAULT,
        }
    }

    /// Sets the dot glyph family (default braille).
    #[must_use]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// Sets the style behind the whole canvas.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Adds a shape (drawn in insertion order).
    #[must_use]
    pub fn shape(mut self, shape: Shape<'a>) -> Self {
        self.shapes.push(shape);
        self
    }

    fn to_dots(&self, plotter: &DotPlotter, x: f64, y: f64) -> Option<(f64, f64)> {
        let span_x = f64::from(plotter.dots_wide().saturating_sub(1));
        let span_y = f64::from(plotter.dots_high().saturating_sub(1));
        Some((
            scale_to_dots(x, self.x_bounds, span_x, false)?,
            scale_to_dots(y, self.y_bounds, span_y, true)?,
        ))
    }
}

impl Widget for Canvas<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        buffer.set_style(area, self.style);
        let plotter = DotPlotter::new(area, self.marker);

        for shape in &self.shapes {
            match *shape {
                Shape::Points { coords, style } => {
                    for &(x, y) in coords {
                        if let Some((dot_x, dot_y)) = self.to_dots(&plotter, x, y) {
                            plotter.set_dot(
                                buffer,
                                super::plot::round_to_i64(dot_x),
                                super::plot::round_to_i64(dot_y),
                                style,
                            );
                        }
                    }
                }
                Shape::Line { from, to, style } => {
                    if let (Some(start), Some(end)) = (
                        self.to_dots(&plotter, from.0, from.1),
                        self.to_dots(&plotter, to.0, to.1),
                    ) {
                        plotter.line(buffer, start, end, style);
                    }
                }
                Shape::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    style,
                } => {
                    let corners = [
                        (x, y),
                        (x + width, y),
                        (x + width, y + height),
                        (x, y + height),
                    ];
                    for index in 0..4 {
                        let (a, b) = (corners[index], corners[(index + 1) % 4]);
                        if let (Some(start), Some(end)) = (
                            self.to_dots(&plotter, a.0, a.1),
                            self.to_dots(&plotter, b.0, b.1),
                        ) {
                            plotter.line(buffer, start, end, style);
                        }
                    }
                }
                Shape::Circle {
                    x,
                    y,
                    radius,
                    style,
                } => {
                    let Some(center) = self.to_dots(&plotter, x, y) else {
                        continue;
                    };
                    let range_x = self.x_bounds[1] - self.x_bounds[0];
                    let range_y = self.y_bounds[1] - self.y_bounds[0];
                    if !radius.is_finite() || range_x <= 0.0 || range_y <= 0.0 {
                        continue;
                    }
                    let span_x = f64::from(plotter.dots_wide().saturating_sub(1));
                    let span_y = f64::from(plotter.dots_high().saturating_sub(1));
                    // A circle in canvas units: each axis scales the radius by its own range.
                    let radius_x = radius.abs() / range_x * span_x;
                    let radius_y = radius.abs() / range_y * span_y;
                    let steps = super::plot::round_to_i64(
                        (radius_x.max(radius_y) * 6.0).clamp(12.0, 4096.0),
                    );
                    let mut previous: Option<(f64, f64)> = None;
                    for step in 0..=steps {
                        let angle = f64::from(u32::try_from(step).unwrap_or(0))
                            / f64::from(u32::try_from(steps).unwrap_or(1))
                            * core::f64::consts::TAU;
                        let point = (
                            center.0 + radius_x * angle.cos(),
                            center.1 - radius_y * angle.sin(),
                        );
                        if let Some(last) = previous {
                            plotter.line(buffer, last, point, style);
                        }
                        previous = Some(point);
                    }
                }
            }
        }
    }
}
