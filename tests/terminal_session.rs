//! Terminal session behavior: inline viewports and hardware cursor control.

use dashboard_tui::backend::{Backend, TestBackend};
use dashboard_tui::core::{
    Buffer, ColumnIndex, Rect, RowIndex, Style, TerminalHeight, TerminalWidth, diff_buffers,
};
use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard, Viewport};

#[test]
fn inline_viewport_draws_in_the_bottom_rows_without_alternate_screen()
-> Result<(), dashboard_tui::error::TuiError> {
    let backend = TestBackend::new(TerminalWidth::new(20), TerminalHeight::new(10));
    let config = TerminalSessionConfig::new().with_viewport(Viewport::Inline(3));
    let mut session = TerminalSessionGuard::enter(backend, config)?;
    assert!(!session.backend().is_alternate_screen_active());

    let area = session.draw(|buffer, area| {
        let _ = buffer.set_string(area.x(), area.y(), "inline!", Style::DEFAULT, 20);
    })?;
    assert_eq!(area, Rect::from_xywh(0, 7, 20, 3));
    let grid = session.backend().to_visual_grid();
    let rows: Vec<&str> = grid.lines().collect();
    assert!(rows[7].starts_with("inline!"), "{grid}");
    assert!(rows[..7].iter().all(|row| row.trim().is_empty()));
    Ok(())
}

#[test]
fn inline_viewport_taller_than_the_terminal_is_clamped()
-> Result<(), dashboard_tui::error::TuiError> {
    let backend = TestBackend::new(TerminalWidth::new(10), TerminalHeight::new(4));
    let config = TerminalSessionConfig::new().with_viewport(Viewport::Inline(50));
    let mut session = TerminalSessionGuard::enter(backend, config)?;
    let area = session.draw(|_, _| {})?;
    assert_eq!(area, Rect::from_xywh(0, 0, 10, 4));
    Ok(())
}

#[test]
fn draw_with_cursor_shows_and_hides_the_hardware_cursor()
-> Result<(), dashboard_tui::error::TuiError> {
    let backend = TestBackend::new(TerminalWidth::new(20), TerminalHeight::new(4));
    let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    assert!(!session.backend().is_cursor_visible());
    session.draw_with_cursor(|_, _| Some((ColumnIndex::new(7), RowIndex::new(1))))?;
    assert!(session.backend().is_cursor_visible());
    assert_eq!(
        session.backend().cursor_position(),
        (ColumnIndex::new(7), RowIndex::new(1))
    );
    session.draw_with_cursor(|_, _| None)?;
    assert!(!session.backend().is_cursor_visible());
    Ok(())
}

#[test]
fn test_backend_scroll_up_moves_rows_and_blanks_the_bottom()
-> Result<(), dashboard_tui::error::TuiError> {
    let mut backend = TestBackend::new(TerminalWidth::new(6), TerminalHeight::new(3));
    let mut frame = Buffer::empty(Rect::from_xywh(0, 0, 6, 3));
    let _ = frame.set_string(
        ColumnIndex::new(0),
        RowIndex::new(0),
        "one",
        Style::DEFAULT,
        6,
    );
    let _ = frame.set_string(
        ColumnIndex::new(0),
        RowIndex::new(1),
        "two",
        Style::DEFAULT,
        6,
    );
    let _ = frame.set_string(
        ColumnIndex::new(0),
        RowIndex::new(2),
        "tre",
        Style::DEFAULT,
        6,
    );
    backend.draw_frame(diff_buffers(&Buffer::empty(Rect::ZERO), &frame))?;
    backend.scroll_up(1)?;
    let grid = backend.to_visual_grid();
    let rows: Vec<&str> = grid.lines().map(str::trim_end).collect();
    assert_eq!(rows, vec!["two", "tre", ""]);
    Ok(())
}
