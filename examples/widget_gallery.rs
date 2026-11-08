//! Gallery of the general-purpose widgets (`examples/widget_gallery.rs`).
//!
//! Renders every standard widget into an in-memory terminal and prints the result, so the
//! whole set can be reviewed (or snapshot-tested) without a TTY:
//!
//! ```bash
//! cargo run --example widget_gallery
//! ```

use std::io::Write;

use dashboard_tui::backend::{Backend, TestBackend};
use dashboard_tui::core::{
    Buffer, Color, Constraint, Layout, Rect, StatefulWidget, Style, Stylize, TerminalHeight,
    TerminalWidth, TextAlignment, Widget, diff_buffers,
};
use dashboard_tui::widgets::{
    Axis, Bar, BarChart, Calendar, Canvas, Chart, Dataset, Gauge, GraphType, LineGauge, List,
    ListState, Paragraph, Row, Shape, Sparkline, Table, TableState, Tabs, WrapMode,
};

fn draw(buffer: &mut Buffer, area: Rect) {
    let [top, middle, bottom] = Layout::vertical([
        Constraint::Fixed(1),
        Constraint::Fixed(8),
        Constraint::Fill(1),
    ])
    .areas(area);

    Tabs::new(["Overview", "Network", "Storage"])
        .select(1)
        .highlight_style(Style::new().bold().on_blue())
        .render(top, buffer);

    let [list_area, table_area, calendar_area] = Layout::horizontal([
        Constraint::Ratio(1, 4),
        Constraint::Fill(1),
        Constraint::Fixed(21),
    ])
    .with_gap(1)
    .areas(middle);

    let mut list_state = ListState::default().with_selected(Some(2));
    List::new(["mount", "sync", "verify", "purge", "restart"])
        .highlight_symbol("> ")
        .highlight_style(Style::new().fg(Color::Cyan).bold())
        .render(list_area, buffer, &mut list_state);

    let mut table_state = TableState::default().with_selected(Some(1));
    Table::new(
        [
            Row::new(["db.tar.zst", "UP", "78%"]),
            Row::new(["spec.pdf", "DOWN", "100%"]),
            Row::new(["master.age", "VERIFY", "100%"]),
        ],
        [
            Constraint::Fill(1),
            Constraint::Fixed(7),
            Constraint::Fixed(5),
        ],
    )
    .header(Row::new(["File", "Mode", "Done"]))
    .column_alignment(2, TextAlignment::Right)
    .highlight_symbol("> ")
    .render(table_area, buffer, &mut table_state);

    Calendar::new(2026, 11)
        .selected(Some(8))
        .mark(14, Style::new().fg(Color::Yellow))
        .render(calendar_area, buffer);

    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(34),
        Constraint::Percentage(33),
        Constraint::Fill(1),
    ])
    .areas(bottom);

    let samples = [2.0, 5.0, 3.0, 8.0, 6.0, 9.0, 4.0, 7.0, 10.0, 6.0, 3.0, 8.0];
    let [gauge_row, line_row, spark_area, text_area] = Layout::vertical([
        Constraint::Fixed(1),
        Constraint::Fixed(1),
        Constraint::Fixed(2),
        Constraint::Fill(1),
    ])
    .areas(left);
    Gauge::default()
        .ratio(0.62)
        .gauge_style(Style::new().bg(Color::Green))
        .label_style(Style::new().fg(Color::Black))
        .render(gauge_row, buffer);
    LineGauge::default()
        .ratio(0.4)
        .filled_style(Style::new().fg(Color::Cyan))
        .render(line_row, buffer);
    Sparkline::new(&samples)
        .style(Style::new().fg(Color::Magenta))
        .render(spark_area, buffer);
    Paragraph::new("Widgets render into any Rect and never allocate while drawing.")
        .wrap(WrapMode::Word)
        .style(Style::new().dim())
        .render(text_area, buffer);

    BarChart::new([Bar::new("cpu", 62), Bar::new("mem", 38), Bar::new("io", 81)])
        .bar_width(5)
        .show_values(true)
        .render(center, buffer);

    let load = [(0.0, 1.0), (1.0, 4.0), (2.0, 2.0), (3.0, 6.0), (4.0, 5.0)];
    let split = Layout::vertical([Constraint::Percentage(55), Constraint::Fill(1)]);
    let [chart_area, canvas_area] = split.areas(right);
    Chart::new([Dataset::new("load", &load).graph_type(GraphType::Line)])
        .x_axis(Axis::new([0.0, 4.0]).labels(["0", "2", "4"]))
        .y_axis(Axis::new([0.0, 6.0]).labels(["0", "3", "6"]))
        .render(chart_area, buffer);
    Canvas::new([0.0, 10.0], [0.0, 10.0])
        .shape(Shape::Circle {
            x: 5.0,
            y: 5.0,
            radius: 4.0,
            style: Style::new().fg(Color::Cyan),
        })
        .shape(Shape::Line {
            from: (0.0, 0.0),
            to: (10.0, 10.0),
            style: Style::new().fg(Color::Red),
        })
        .render(canvas_area, buffer);
}

fn main() -> Result<(), dashboard_tui::error::TuiError> {
    let (width, height) = (TerminalWidth::new(110), TerminalHeight::new(22));
    let mut backend = TestBackend::new(width, height);
    let viewport = Rect::from_size(width, height);
    let mut frame = Buffer::empty(viewport);
    draw(&mut frame, viewport);
    backend.draw_frame(diff_buffers(&Buffer::empty(Rect::ZERO), &frame))?;
    backend.flush()?;
    writeln!(std::io::stdout(), "{}", backend.to_visual_grid())?;
    Ok(())
}
