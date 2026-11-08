//! Top 1-line header bar and bottom 1-line footer bar renderer (`chrome`).
//!
//! Directly ports `renderHeader` and `renderFooter` from `ref/dashboard/de_dashboard.go`:
//! - **Top header**: App title + semantic status badge + priority-fitted telemetry
//!   segments on the left; `F1`..`F12` page tabs on the right.
//! - **Bottom footer**: Displays active toast/flash notification (`✓` / `✗`), or
//!   pending multi-digit jump indicator (`#1_`), or priority-fitted [`KeyHint`] strip.

use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::display_width;
use crate::core::palette::ColorPalette;
use crate::core::priority_strip::{KeyHint, render_fitted_hints, render_fitted_segments};
use crate::core::style::Style;
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::page::Page;

/// Writes the active tab label `" [F1:Monitor] "`, or `" [F1:Monitor <badge>] "` when a badge is set.
fn write_active_tab_label<B: DashboardBackend>(
    out: &mut StackStr<96>,
    page: &Page<B>,
    badge: &str,
) {
    if badge.is_empty() {
        let _ = write!(out, " [{}:{}] ", page.key_hint(), page.title());
    } else {
        let _ = write!(out, " [{}:{} {}] ", page.key_hint(), page.title(), badge);
    }
}

/// Measures the total column width of the right-aligned page tab strip.
///
/// Active tab: `" [F1:Monitor] "` (plus `" <badge>"` when set); inactive: `" F2:Settings "`.
fn measure_page_tabs_width<B: DashboardBackend>(
    pages: &[Page<B>],
    active_page: usize,
    active_badge: &str,
) -> usize {
    let badge_width = if active_badge.is_empty() {
        0
    } else {
        display_width(active_badge).as_usize().saturating_add(1)
    };
    pages.iter().enumerate().fold(0_usize, |total, (idx, p)| {
        let decoration = if idx == active_page {
            4_usize.saturating_add(badge_width)
        } else {
            2
        };
        let tab_len = display_width(p.key_hint())
            .as_usize()
            .saturating_add(1)
            .saturating_add(display_width(p.title()).as_usize())
            .saturating_add(decoration);
        total.saturating_add(tab_len)
    })
}

/// Renders the 1-line top dashboard header bar at `row` across `width` columns.
#[expect(
    clippy::cast_possible_truncation,
    clippy::too_many_arguments,
    reason = "widths bounded by u16; internal header bar renderer"
)]
pub fn render_header_bar<B: DashboardBackend>(
    backend: &B,
    pages: &[Page<B>],
    active_page: usize,
    buffer: &mut Buffer,
    col: ColumnIndex,
    row: RowIndex,
    width: u16,
    palette: &ColorPalette,
) {
    if width < 8 {
        return;
    }
    let right_edge = col.get().saturating_add(width);

    // Measure right-aligned page tabs first so we know how much space remains for telemetry
    let active_badge = pages
        .get(active_page)
        .map(|p| p.tab_badge(backend))
        .unwrap_or_default();
    let tabs_width = measure_page_tabs_width(pages, active_page, &active_badge);

    let app_title = backend.app_title();
    let (badge_text, badge_color) = backend.status_badge();
    let title_style = Style::new().with_fg(palette.accent).bold();
    let sep_style = Style::new().with_fg(palette.dim);
    let badge_style = Style::new().with_fg(badge_color).bold();

    let mut cx = buffer.set_string(col, row, " ", sep_style, 1);
    cx = buffer.set_string(
        cx,
        row,
        app_title,
        title_style,
        right_edge.saturating_sub(cx.get()),
    );

    if !badge_text.is_empty() && cx.get().saturating_add(4) < right_edge {
        cx = buffer.set_string(
            cx,
            row,
            " │ ",
            sep_style,
            right_edge.saturating_sub(cx.get()),
        );
        cx = buffer.set_string(
            cx,
            row,
            badge_text,
            badge_style,
            right_edge.saturating_sub(cx.get()),
        );
    }

    let tabs_fit = tabs_width.saturating_add(4) <= usize::from(right_edge.saturating_sub(cx.get()));
    let telemetry_limit = if tabs_fit {
        right_edge
            .saturating_sub(tabs_width as u16)
            .saturating_sub(2)
    } else {
        right_edge
    };

    let segments = backend.header_segments();
    if !segments.is_empty() && cx.get().saturating_add(5) < telemetry_limit {
        cx = buffer.set_string(
            cx,
            row,
            " │ ",
            sep_style,
            telemetry_limit.saturating_sub(cx.get()),
        );
        let avail_seg_w = telemetry_limit.saturating_sub(cx.get());
        let _ = render_fitted_segments(
            buffer,
            cx,
            row,
            TerminalWidth::new(avail_seg_w),
            &segments,
            sep_style,
        );
    }

    if tabs_fit && !pages.is_empty() {
        let mut tx = ColumnIndex::new(right_edge.saturating_sub(tabs_width as u16));
        for (idx, p) in pages.iter().enumerate() {
            let mut tab_str = StackStr::<96>::new();
            if idx == active_page {
                write_active_tab_label(&mut tab_str, p, &active_badge);
                let active_style = Style::new().with_fg(palette.accent2).bold();
                tx = buffer.set_string(
                    tx,
                    row,
                    tab_str.as_str(),
                    active_style,
                    right_edge.saturating_sub(tx.get()),
                );
            } else {
                let _ = write!(tab_str, " {}:{} ", p.key_hint(), p.title());
                let inactive_style = Style::new().with_fg(palette.muted);
                tx = buffer.set_string(
                    tx,
                    row,
                    tab_str.as_str(),
                    inactive_style,
                    right_edge.saturating_sub(tx.get()),
                );
            }
        }
    }
}

/// Renders the 1-line bottom dashboard footer bar at `row` across `width` columns.
#[expect(clippy::too_many_arguments, reason = "internal footer bar renderer")]
pub fn render_footer_bar(
    buffer: &mut Buffer,
    col: ColumnIndex,
    row: RowIndex,
    width: u16,
    palette: &ColorPalette,
    hints: &[KeyHint<'_>],
    toast: Option<(&str, bool)>,
    pending_digit: Option<u16>,
    tty_mode: bool,
) {
    if width < 8 {
        return;
    }
    let right_edge = col.get().saturating_add(width);

    if let Some((msg, is_err)) = toast {
        let icon = if is_err {
            if tty_mode { " [!] " } else { " ✗ " }
        } else if tty_mode {
            " [*] "
        } else {
            " ✓ "
        };
        let color = if is_err { palette.err } else { palette.ok };
        let style = Style::new().with_fg(color).bold();
        let cx = buffer.set_string(col, row, icon, style, width);
        let _ = buffer.set_string(cx, row, msg, style, right_edge.saturating_sub(cx.get()));
        return;
    }

    let mut cx = col.saturating_add(1);
    if let Some(d) = pending_digit {
        let mut jump_tag = StackStr::<16>::new();
        let _ = write!(jump_tag, "#{d}_  ");
        let jump_style = Style::new().with_fg(palette.accent2).bold();
        cx = buffer.set_string(
            cx,
            row,
            jump_tag.as_str(),
            jump_style,
            right_edge.saturating_sub(cx.get()),
        );
    }

    let avail = right_edge.saturating_sub(cx.get());
    let _ = render_fitted_hints(
        buffer,
        cx,
        row,
        TerminalWidth::new(avail),
        hints,
        Style::new().with_fg(palette.muted),
    );
}
