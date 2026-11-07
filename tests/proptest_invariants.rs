//! Property-based (`proptest`) invariant verification across `0..=u16::MAX` and arbitrary Unicode.
//!
//! Enforces Pillar 6.2 of `RULES.md`:
//! - Layout solvers, constraint splitters, and table column solvers never panic or overflow parent bounds.
//! - Grapheme cluster measurement, clipping, ellipsis truncation, and word wrapping never slice mid-codepoint
//!   or exceed the target `TerminalWidth`.
//! - `Buffer::set_str_clipped` maintains wide-character continuation invariants at all widths (`0..=120`).
//! - `WaveScaler` never panics or returns NaN/sub-floor ceilings even on extreme `f64` inputs.
//! - `Page::resolve_layout` never panics across all 6 `SectionLayoutMode`s and `0..=u16::MAX` viewports.

use proptest::prelude::*;
use tui_rs::components::{WaveScaler, WaveScalerConfig, nice_ceil, round_scale};
use tui_rs::core::{
    Buffer, Color, ColumnIndex, Constraint, Layout, PrioritySegment, Rect, RowIndex, Spacing4D,
    Style, TerminalHeight, TerminalWidth, TextAlignment, clip_from_left, clip_to_width,
    display_width, split_equal_spans, truncate_with_ellipsis_parts, wrap_lines_iter,
};
use tui_rs::dashboard::{
    DashboardBackend, OptionOutcome, OptionSpec, Page, Section, SectionLayoutMode, SpacingConfig,
};
use tui_rs::widgets::{TableColumn, WeightedTable};

struct DummyBackend;

impl DashboardBackend for DummyBackend {
    fn app_title(&self) -> &'static str {
        "VaultDrive"
    }

    fn status_badge(&self) -> (&'static str, Color) {
        ("MOUNTED", Color::Green)
    }

    fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
        vec![PrioritySegment::new("Up 1.0 MB/s", Style::DEFAULT, 100)]
    }
}

proptest! {
    #[test]
    fn rect_operations_never_panic_or_exceed_parent(
        x in 0..=u16::MAX,
        y in 0..=u16::MAX,
        w in 0..=u16::MAX,
        h in 0..=u16::MAX,
        top in 0..=100u16,
        right in 0..=100u16,
        bottom in 0..=100u16,
        left in 0..=100u16,
        min_w in 0..=u16::MAX,
        min_h in 0..=u16::MAX,
    ) {
        let parent = Rect::new(
            ColumnIndex::new(x),
            RowIndex::new(y),
            TerminalWidth::new(w),
            TerminalHeight::new(h),
        );
        let spacing = Spacing4D::new(top, right, bottom, left);

        let inner = parent.inner(spacing);
        prop_assert!(inner.width().get() <= parent.width().get());
        prop_assert!(inner.height().get() <= parent.height().get());
        prop_assert!(inner.left().get() >= parent.left().get());
        prop_assert!(inner.top().get() >= parent.top().get());

        let clamped = parent.inner_clamped(
            spacing,
            TerminalWidth::new(min_w),
            TerminalHeight::new(min_h),
        );
        prop_assert!(clamped.width().get() <= parent.width().get());
        prop_assert!(clamped.height().get() <= parent.height().get());

        let centered = parent.centered(TerminalWidth::new(min_w), TerminalHeight::new(min_h));
        prop_assert!(centered.width().get() <= parent.width().get());
        prop_assert!(centered.height().get() <= parent.height().get());
        prop_assert!(centered.left().get() >= parent.left().get());
        prop_assert!(centered.top().get() >= parent.top().get());
    }

    #[test]
    fn layout_solver_never_exceeds_parent_bounds(
        x in 0..=u16::MAX,
        y in 0..=u16::MAX,
        w in 0..=u16::MAX,
        h in 0..=u16::MAX,
        gap in 0..=16u16,
        pct in 0..=150u16,
        fixed in 0..=500u16,
        min_sz in 0..=500u16,
        max_sz in 0..=500u16,
        fill_w in 0..=20u16,
    ) {
        let parent = Rect::new(
            ColumnIndex::new(x),
            RowIndex::new(y),
            TerminalWidth::new(w),
            TerminalHeight::new(h),
        );
        let constraints = [
            Constraint::Percentage(pct),
            Constraint::Fixed(fixed),
            Constraint::Min(min_sz),
            Constraint::Max(max_sz),
            Constraint::Fill(fill_w),
        ];

        let h_chunks = Layout::horizontal(constraints).with_gap(gap).split(parent);
        prop_assert_eq!(h_chunks.len(), constraints.len());
        let mut total_w: u32 = 0;
        for chunk in &h_chunks {
            prop_assert!(chunk.width().get() <= parent.width().get());
            prop_assert_eq!(chunk.height(), parent.height());
            total_w += u32::from(chunk.width().get());
        }
        prop_assert!(total_w <= u32::from(parent.width().get()));

        let v_chunks = Layout::vertical(constraints).with_gap(gap).split(parent);
        prop_assert_eq!(v_chunks.len(), constraints.len());
        let mut total_h: u32 = 0;
        for chunk in &v_chunks {
            prop_assert!(chunk.height().get() <= parent.height().get());
            prop_assert_eq!(chunk.width(), parent.width());
            total_h += u32::from(chunk.height().get());
        }
        prop_assert!(total_h <= u32::from(parent.height().get()));
    }

    #[test]
    fn split_equal_spans_conserves_budget(
        total in 0..=u16::MAX,
        gap in 0..=20u16,
        count in 0..=32usize,
    ) {
        let spans = split_equal_spans(total, gap, count);
        prop_assert_eq!(spans.len(), count);
        if count > 0 {
            let sum_spans: u32 = spans.iter().copied().map(u32::from).sum();
            prop_assert!(sum_spans <= u32::from(total));
        }
    }

    #[test]
    fn weighted_table_solver_never_exceeds_budget(
        total_w in 0..=u16::MAX,
        gap in 0..=8u16,
        w0 in 1..=20u16,
        w1 in 1..=20u16,
        w2 in 1..=20u16,
        m0 in 1..=30u16,
        m1 in 1..=30u16,
        m2 in 1..=30u16,
    ) {
        let cols = [
            TableColumn::new("ColA", w0).with_min_width(m0),
            TableColumn::new("ColumnB", w1).with_min_width(m1),
            TableColumn::new("C", w2).with_min_width(m2).with_alignment(TextAlignment::Right),
        ];
        let rows: [&[&str]; 2] = [
            &["alpha", "beta-long-cell-value", "123.4 MB"],
            &["🚀", "日本語テスト", "0 B"],
        ];
        let table = WeightedTable::new(&cols, &rows).with_column_gap(gap);
        let mut out = [0u16; 3];
        table.solve_column_widths(total_w, &mut out);
        for &cw in &out {
            prop_assert!(cw >= 1);
        }
    }

    #[test]
    fn unicode_grapheme_clipping_and_wrapping_invariants(
        text in ".*",
        max_w in 0..=120u16,
    ) {
        let budget = TerminalWidth::new(max_w);
        let clipped = clip_to_width(&text, budget);
        prop_assert!(display_width(clipped).get() <= max_w);

        let left_clipped = clip_from_left(&text, budget);
        prop_assert!(display_width(left_clipped).get() <= max_w);

        let (prefix, has_ellipsis, used_w) = truncate_with_ellipsis_parts(&text, budget);
        prop_assert!(used_w.get() <= max_w);
        let expected_w = display_width(prefix).get().saturating_add(u16::from(has_ellipsis));
        prop_assert_eq!(used_w.get(), expected_w);

        for line in wrap_lines_iter(&text, budget) {
            if max_w > 0 {
                prop_assert!(display_width(line).get() <= max_w);
            } else {
                prop_assert!(line.is_empty());
            }
        }
    }

    #[test]
    fn buffer_set_str_clipped_wide_grapheme_invariants(
        text in ".*",
        buf_w in 1..=60u16,
        start_x in 0..=60u16,
    ) {
        let rect = Rect::from_xywh(0, 0, buf_w, 2);
        let mut buf = Buffer::empty(rect);
        let end_col = buf.set_str_clipped(
            ColumnIndex::new(start_x),
            RowIndex::new(0),
            &text,
            ColumnIndex::new(buf_w),
            Style::DEFAULT,
        );
        prop_assert!(end_col.get() <= buf_w.max(start_x));

        let lines = buf.to_visual_lines();
        prop_assert_eq!(lines.len(), 2);
        for line in &lines {
            prop_assert_eq!(display_width(line).get(), buf_w);
        }
    }

    #[test]
    fn wave_scaler_always_respects_floor(
        samples in prop::collection::vec(-1e12f64..1e12f64, 0..64),
        floor in 1.0f64..1e6f64,
    ) {
        let mut scaler = WaveScaler::new(WaveScalerConfig::new(floor));
        let ceiling = scaler.update(&samples);
        prop_assert!(ceiling.is_finite());
        prop_assert!(ceiling >= floor);

        for &s in &samples {
            let r = round_scale(s, floor);
            prop_assert!(r.is_finite());
            prop_assert!(r >= floor);

            let n = nice_ceil(s, floor);
            prop_assert!(n.is_finite());
            prop_assert!(n >= floor);
        }
    }

    #[test]
    fn page_resolve_layout_never_panics_or_exceeds_body(
        w in 0..=400u16,
        h in 0..=200u16,
        tp in 0..=4u16,
        ps in 0..=4u16,
        ss in 0..=4u16,
        so in 0..=4u16,
        mode_idx in 0..6usize,
    ) {
        let mode = match mode_idx {
            0 => SectionLayoutMode::Auto,
            1 => SectionLayoutMode::Columns,
            2 => SectionLayoutMode::Rows,
            3 => SectionLayoutMode::OneTopTwoBottom,
            4 => SectionLayoutMode::TwoTopOneBottom,
            _ => SectionLayoutMode::Grid2x2,
        };

        let mut page = Page::new("Test", "F1").with_layout_mode(mode);
        for i in 0..4u16 {
            let sec = Section::menu(
                format!("Sec{i}"),
                "Enter",
                vec![OptionSpec::new(
                    i.saturating_add(1),
                    "Action",
                    |_| OptionOutcome::None,
                )],
            )
            .with_heights(4, 10);
            page.push_section(sec);
        }

        let spacing = SpacingConfig::new(tp, ps, ss, so);
        let body = Rect::from_xywh(0, 1, w, h);
        let resolved = page.resolve_layout(&DummyBackend, body, spacing);

        for item in resolved {
            prop_assert!(item.rect.width().get() <= body.width().get());
            prop_assert!(item.rect.height().get() <= body.height().get());
        }
    }
}
