//! Property-based robustness tests for the general-purpose widget layer.
//!
//! Every widget must render into arbitrary (including degenerate, oversized, and partly
//! off-buffer) areas with arbitrary data without panicking or overflowing.

use proptest::prelude::*;
use tui_rs::core::{Buffer, Constraint, Line, Rect, StatefulWidget, Style, TextAlignment, Widget};
use tui_rs::widgets::{
    Bar, BarChart, Gauge, LineGauge, List, ListState, Paragraph, Row, Sparkline, Table, TableState,
    Tabs, WrapMode,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn list_and_table_never_panic(
        x in 0..=300u16, y in 0..=300u16, w in 0..=120u16, h in 0..=60u16,
        buf_w in 0..=200u16, buf_h in 0..=80u16,
        count in 0..=200usize,
        selected in prop::option::of(0..=usize::MAX),
        offset in 0..=usize::MAX,
        padding in 0..=100usize,
        symbol in prop::sample::select(vec!["", ">", ">> ", "😀 "]),
    ) {
        let area = Rect::from_xywh(x, y, w, h);
        let mut buffer = Buffer::empty(Rect::from_xywh(0, 0, buf_w, buf_h));
        let labels: Vec<String> = (0..count).map(|i| format!("item {i}")).collect();

        let mut list_state = ListState::default().with_selected(selected);
        list_state.set_offset(offset);
        let list = List::new(labels.iter().map(String::as_str))
            .highlight_symbol(symbol)
            .scroll_padding(padding);
        list.render(area, &mut buffer, &mut list_state);
        if count > 0 {
            if let Some(index) = list_state.selected() { prop_assert!(index < count); }
            prop_assert!(list_state.offset() < count);
        }

        let mut table_state = TableState::default().with_selected(selected);
        table_state.set_offset(offset);
        let table = Table::new(
            labels.iter().map(|label| Row::new([label.as_str(), "x", "yy"])),
            [Constraint::Fill(1), Constraint::Fixed(3), Constraint::Percentage(20)],
        )
        .header(Row::new(["a", "b", "c"]))
        .footer(Row::new(["f"]))
        .highlight_symbol(symbol)
        .column_alignment(1, TextAlignment::Right)
        .scroll_padding(padding);
        table.render(area, &mut buffer, &mut table_state);
        if count > 0 {
            if let Some(index) = table_state.selected() { prop_assert!(index < count); }
        }
    }

    #[test]
    fn text_widgets_never_panic(
        x in 0..=300u16, y in 0..=300u16, w in 0..=120u16, h in 0..=40u16,
        buf_w in 0..=200u16, buf_h in 0..=80u16,
        text in "[a-zA-Z0-9 \\n中文😀]{0,60}",
        wrap_pick in 0..3u8,
        align_pick in 0..3u8,
        scroll_rows in 0..=100u16, scroll_cols in 0..=100u16,
        selected in 0..=50usize,
    ) {
        let area = Rect::from_xywh(x, y, w, h);
        let mut buffer = Buffer::empty(Rect::from_xywh(0, 0, buf_w, buf_h));
        let wrap = match wrap_pick { 0 => WrapMode::None, 1 => WrapMode::Char, _ => WrapMode::Word };
        let alignment = match align_pick { 0 => TextAlignment::Left, 1 => TextAlignment::Center, _ => TextAlignment::Right };

        let paragraph = Paragraph::new(text.clone())
            .wrap(wrap)
            .alignment(alignment)
            .scroll(scroll_rows, scroll_cols);
        let counted = paragraph.line_count(w);
        paragraph.render(area, &mut buffer);
        if wrap != WrapMode::None && w > 0 {
            // A wrapped paragraph never needs more rows than it has characters (+ blank lines).
            prop_assert!(counted <= text.chars().count().max(1) + text.matches('\n').count() + 1);
        }

        Tabs::new(text.split(' ').map(Line::raw)).select(selected).render(area, &mut buffer);
    }

    #[test]
    fn gauges_and_charts_never_panic(
        x in 0..=300u16, y in 0..=300u16, w in 0..=120u16, h in 0..=40u16,
        buf_w in 0..=200u16, buf_h in 0..=80u16,
        ratio in prop::num::f64::ANY,
        data in prop::collection::vec(prop::num::f64::ANY, 0..=200),
        bars in prop::collection::vec(0..=u64::MAX, 0..=40),
        bar_width in 0..=10u16, bar_gap in 0..=10u16,
        max in prop::option::of(0..=u64::MAX),
        unicode in any::<bool>(),
    ) {
        let area = Rect::from_xywh(x, y, w, h);
        let mut buffer = Buffer::empty(Rect::from_xywh(0, 0, buf_w, buf_h));

        Gauge::default().ratio(ratio).use_unicode(unicode).render(area, &mut buffer);
        LineGauge::default().ratio(ratio).tty_mode(unicode).render(area, &mut buffer);
        Sparkline::new(&data).style(Style::new().bold()).render(area, &mut buffer);
        Sparkline::new(&data).max(1.0).tty_mode(true).render(area, &mut buffer);

        let mut chart = BarChart::new(bars.iter().map(|&v| Bar::new("lbl", v)))
            .bar_width(bar_width)
            .bar_gap(bar_gap)
            .show_values(true);
        if let Some(limit) = max { chart = chart.max(limit); }
        chart.render(area, &mut buffer);
    }
}
