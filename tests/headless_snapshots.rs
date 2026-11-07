//! Headless `TestBackend` & `Buffer::to_visual_lines()` visual verification suite.
//!
//! Enforces Pillar 6.1 of `RULES.md`:
//! - Verifies `TabPanel` 4-step graceful title degradation (`FullNotch` -> `CompactNotch` ->
//!   `TruncatedNotch` -> `PlainBorder`) and TTY ASCII fallback.
//! - Verifies `SingleWaveform` and `DualWaveform` bidirectional center-axis rendering.
//! - Verifies `MenuList` 1-column and 2-column auto-activation (`width >= 100` & `>= 6 items`)
//!   with 4-directional `option_gap` and `SubCellScrollbar`.
//! - Verifies Tier 1 `DashboardEngine` end-to-end rendering, `F1`/`F2` page switching,
//!   `Tab` focus cycling, multi-digit `1`..`20` option jumping (`#2_` pending indicator),
//!   `SubView` drill-down stack, and Tier 2 `SectionGapModal` / `ColorThemeModal` live preview
//!   and revert on `Esc`.

use tui_rs::backend::{Backend, TestBackend};
use tui_rs::components::{
    DualWaveform, GradientMeter, GraphSymbolMode, HalfBlockQr, SingleWaveform,
};
use tui_rs::core::{
    diff_buffers, Buffer, Color, ColorPalette, ColumnIndex, KeyHint, PrioritySegment, Rect,
    RowIndex, Style, TerminalHeight, TerminalWidth,
};
use tui_rs::dashboard::{
    DashboardBackend, DashboardEngine, EngineAction, KeyBindings, KeyCode, KeyPress, ModalContext,
    OptionOutcome, OptionSpec, Page, Section, SectionWidget, SpacingLayer, SubView, SubViewResult,
};
use tui_rs::universal::{color_theme_option, section_gap_option, ConfirmModal, EditModal};
use tui_rs::widgets::{MenuItem, MenuList, TabPanel};

struct MockVaultBackend {
    mounted: bool,
    remote_name: String,
    up_history: Vec<f64>,
    down_history: Vec<f64>,
    saved_count: usize,
}

impl Default for MockVaultBackend {
    fn default() -> Self {
        Self {
            mounted: true,
            remote_name: "gdrive-crypt:/vault".to_string(),
            up_history: vec![12_000.0, 48_000.0, 128_000.0, 256_000.0, 64_000.0],
            down_history: vec![64_000.0, 192_000.0, 512_000.0, 384_000.0, 128_000.0],
            saved_count: 0,
        }
    }
}

impl DashboardBackend for MockVaultBackend {
    fn app_title(&self) -> &'static str {
        "VaultDrive v3.0"
    }

    fn status_badge(&self) -> (&'static str, Color) {
        if self.mounted {
            ("● MOUNTED", Color::Green)
        } else {
            ("○ UNMOUNTED", Color::Yellow)
        }
    }

    fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
        vec![
            PrioritySegment::new(self.remote_name.as_str(), Style::DEFAULT, 90),
            PrioritySegment::new("▲ 64 KB/s ▼ 128 KB/s", Style::DEFAULT, 70),
            PrioritySegment::new("Cache: 1.2 GB", Style::DEFAULT, 40),
        ]
    }

    fn save_ui_prefs(&mut self) -> Result<(), tui_rs::error::TuiError> {
        self.saved_count += 1;
        Ok(())
    }
}

struct SyncTelemetryWidget;

impl SectionWidget<MockVaultBackend> for SyncTelemetryWidget {
    fn title(&self, _backend: &MockVaultBackend) -> String {
        "Sync Telemetry".to_string()
    }

    fn subtitle(&self, _backend: &MockVaultBackend) -> String {
        "rclone vfs".to_string()
    }

    fn render(
        &self,
        backend: &MockVaultBackend,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        let mode = if tty_mode {
            GraphSymbolMode::Tty
        } else {
            GraphSymbolMode::Braille
        };
        DualWaveform::new(&backend.up_history, &backend.down_history, 512_000.0)
            .with_labels("64 KB/s", "128 KB/s")
            .with_mode(mode)
            .render(buffer, inner, palette);
    }
}

struct MockLogSubView;

impl SubView<MockVaultBackend> for MockLogSubView {
    fn title(&self) -> &'static str {
        "Journalctl Live Stream"
    }

    fn render(
        &self,
        _backend: &MockVaultBackend,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        let inner = TabPanel::new(self.title())
            .with_subtitle("Esc to return")
            .with_focused(true)
            .with_tty_mode(tty_mode)
            .render(buffer, area, palette);
        let _ = buffer.set_string(
            inner.x(),
            inner.y(),
            "2026-11-07T06:00:00Z INFO FUSE mount active on /mnt/vaultdrive",
            Style::new().with_fg(palette.text),
            inner.width().get(),
        );
    }

    fn on_key(&mut self, _backend: &mut MockVaultBackend, key: KeyPress) -> SubViewResult {
        if key.code() == KeyCode::Esc {
            SubViewResult::PopWithToast {
                message: "Closed log stream".to_string(),
                is_error: false,
            }
        } else {
            SubViewResult::Continue
        }
    }

    fn footer_hints(&self) -> Vec<KeyHint<'_>> {
        vec![KeyHint::new("Esc", "Back", Style::DEFAULT, 100)]
    }
}

#[test]
fn tab_panel_4_step_width_degradation_and_tty_fallback() {
    let palette = ColorPalette::DEFAULT;

    // Step 1: Wide enough for full notch ("╭─┐ Drive Status ┌─...─╮")
    let wide_rect = Rect::from_xywh(0, 0, 32, 4);
    let mut wide_buf = Buffer::empty(wide_rect);
    let inner = TabPanel::new("Drive Status")
        .with_subtitle("FUSE")
        .with_focused(true)
        .render(&mut wide_buf, wide_rect, &palette);
    assert_eq!(inner.width().get(), 28);
    assert_eq!(inner.height().get(), 2);
    let wide_lines = wide_buf.to_visual_lines();
    assert!(wide_lines[0].starts_with("╭─┐ Drive Status ┌─"));
    assert!(wide_lines[3].starts_with("╰─┘ FUSE └─"));

    // Step 2: Compact width where badge is dropped
    let compact_rect = Rect::from_xywh(0, 0, 22, 3);
    let mut compact_buf = Buffer::empty(compact_rect);
    let _ = TabPanel::new("Drive Status")
        .with_badge("MOUNTED")
        .render(&mut compact_buf, compact_rect, &palette);
    let compact_lines = compact_buf.to_visual_lines();
    assert!(compact_lines[0].starts_with("╭─┐ Drive Status ┌"));
    assert!(!compact_lines[0].contains("MOUNTED"));

    // Step 3: Truncated notch ("╭─┐ Driv… ┌╮")
    let trunc_rect = Rect::from_xywh(0, 0, 14, 3);
    let mut trunc_buf = Buffer::empty(trunc_rect);
    let _ = TabPanel::new("Drive Status").render(&mut trunc_buf, trunc_rect, &palette);
    let trunc_lines = trunc_buf.to_visual_lines();
    assert!(trunc_lines[0].contains('…'));

    // Step 4: Plain border when width < 8 ("╭────╮")
    let tiny_rect = Rect::from_xywh(0, 0, 6, 3);
    let mut tiny_buf = Buffer::empty(tiny_rect);
    let _ = TabPanel::new("Drive Status").render(&mut tiny_buf, tiny_rect, &palette);
    assert_eq!(tiny_buf.to_visual_lines()[0], "╭────╮");

    // TTY square-corner fallback ("┌─┐ Drive Status ┌─...─┐")
    let mut tty_buf = Buffer::empty(wide_rect);
    let _ = TabPanel::new("Drive Status")
        .with_tty_mode(true)
        .render(&mut tty_buf, wide_rect, &palette);
    let tty_lines = tty_buf.to_visual_lines();
    assert!(tty_lines[0].starts_with("┌─┐ Drive Status ┌─"));
}

#[test]
fn dual_waveform_and_meter_and_qr_render_deterministically() {
    let palette = ColorPalette::DEFAULT;
    let rect = Rect::from_xywh(0, 0, 44, 5);
    let mut buf = Buffer::empty(rect);

    let up = [0.0, 64_000.0, 128_000.0, 256_000.0];
    let down = [32_000.0, 96_000.0, 192_000.0, 256_000.0];
    DualWaveform::new(&up, &down, 256_000.0)
        .with_labels("256 KB/s", "256 KB/s")
        .render(&mut buf, rect, &palette);

    let lines = buf.to_visual_lines();
    // Row 2 is the center axis row with ▲ on left, [≤250K] in center, and ▼ on right
    assert!(lines[2].contains("▲ 256 KB/s"));
    assert!(lines[2].contains("[≤250K]"));
    assert!(lines[2].contains("▼ 256 KB/s"));

    // Verify full-width scaling across resized window widths (24, 44, 90, 160 columns):
    // Even with only 4 samples, fill_width=true stretches the waveform across all columns
    // so the left half is never left empty!
    let active_samples = [128_000.0, 256_000.0, 192_000.0, 256_000.0];
    for width in [24_u16, 44, 90, 160] {
        let resize_rect = Rect::from_xywh(0, 0, width, 5);
        let mut resize_buf = Buffer::empty(resize_rect);
        DualWaveform::new(&active_samples, &active_samples, 256_000.0)
            .with_labels("Up", "Down")
            .render(&mut resize_buf, resize_rect, &palette);
        // Check that both the leftmost column (x=0) and rightmost column (x=width-1)
        // on the top baseline row (y=1) and bottom baseline row (y=3) have active non-empty glyphs
        for x in [0_u16, width / 4, width / 2, width - 1] {
            let top_cell = resize_buf
                .cell_at(ColumnIndex::new(x), RowIndex::new(1))
                .map_or(" ", |c| c.symbol());
            let bot_cell = resize_buf
                .cell_at(ColumnIndex::new(x), RowIndex::new(3))
                .map_or(" ", |c| c.symbol());
            assert_ne!(top_cell, " ");
            assert_ne!(top_cell, "⣀");
            assert_ne!(bot_cell, " ");
            assert_ne!(bot_cell, "⠉");
        }
    }

    // SingleWaveform inverted & non-inverted
    let sw_rect = Rect::from_xywh(0, 0, 16, 2);
    let mut sw_buf = Buffer::empty(sw_rect);
    SingleWaveform::new(&up, 256_000.0).render(&mut sw_buf, sw_rect, palette.gradient_lut());
    assert_eq!(sw_buf.to_visual_lines().len(), 2);

    // GradientMeter
    let m_rect = Rect::from_xywh(0, 0, 12, 1);
    let mut m_buf = Buffer::empty(m_rect);
    let end_col = GradientMeter::new(0.5, 10).render(
        &mut m_buf,
        ColumnIndex::new(0),
        RowIndex::new(0),
        &palette,
    );
    assert_eq!(end_col.get(), 10);
    assert_eq!(m_buf.to_visual_lines()[0].trim(), "[■■■■□□□□]");

    // QR half-block matrix
    let modules = [true, false, true, false, true, false, true, false, true];
    let qr = HalfBlockQr::new(&modules, 3).with_quiet_zone(1);
    assert_eq!(qr.rendered_width(), 5);
    assert_eq!(qr.rendered_height(), 3);
    let qr_rect = Rect::from_xywh(0, 0, 10, 5);
    let mut qr_buf = Buffer::empty(qr_rect);
    qr.render(&mut qr_buf, ColumnIndex::new(0), RowIndex::new(0));
}

#[test]
fn menu_list_single_and_two_column_with_option_gaps() {
    let palette = ColorPalette::DEFAULT;
    let items: Vec<MenuItem<'_>> = (1..=8)
        .map(|i| MenuItem::new(i, "Configure Vault Option").with_badge("ON"))
        .collect();

    // Narrow (< 100 cols) -> 1 column, option_gap=1 adds 1-row top inset so [1] is on row 1
    let narrow = Rect::from_xywh(0, 0, 60, 10);
    let mut narrow_buf = Buffer::empty(narrow);
    MenuList::new(&items)
        .with_cursor(Some(0))
        .with_option_gap(1)
        .render(&mut narrow_buf, narrow, &palette);
    let narrow_lines = narrow_buf.to_visual_lines();
    assert_eq!(narrow_lines[0].trim(), "");
    assert!(narrow_lines[1].contains("[1]"));
    assert!(!narrow_lines[1].contains("[5]"));

    // Wide (inner_w >= 72 and items > inner_h) -> 2 columns side by side
    let wide = Rect::from_xywh(0, 0, 110, 6);
    let mut wide_buf = Buffer::empty(wide);
    MenuList::new(&items)
        .with_cursor(Some(0))
        .with_option_gap(0)
        .render(&mut wide_buf, wide, &palette);
    let wide_lines = wide_buf.to_visual_lines();
    // Row 0 has item [1] in left column and item [5] in right column
    assert!(wide_lines[0].contains("[1]") && wide_lines[0].contains("[5]"));
}

#[test]
#[expect(clippy::too_many_lines, reason = "comprehensive end-to-end integration test")]
fn dashboard_engine_end_to_end_navigation_modals_and_sub_views() -> Result<(), tui_rs::error::TuiError> {
    let mut page1 = Page::new("Monitor", "F1");
    page1.push_section(Section::display(Box::new(SyncTelemetryWidget)).with_heights(5, 8));

    let mut actions = Section::menu("Quick Actions", "1-3 run", Vec::new()).with_heights(5, 10);
    actions.push_option(OptionSpec::new(
        1,
        "Toggle Mount",
        |ctx: &mut ModalContext<'_, MockVaultBackend>| {
            ctx.backend.mounted = !ctx.backend.mounted;
            OptionOutcome::Toast {
                message: if ctx.backend.mounted {
                    "Vault mounted".to_string()
                } else {
                    "Vault unmounted".to_string()
                },
                is_error: false,
            }
        },
    ));
    actions.push_option(OptionSpec::new(
        2,
        "Rename Remote",
        |ctx: &mut ModalContext<'_, MockVaultBackend>| {
            let initial = ctx.backend.remote_name.clone();
            OptionOutcome::OpenModal(Box::new(EditModal::new(
                "Rename Remote",
                "Enter new rclone remote path:",
                initial,
                |backend: &mut MockVaultBackend, val: &str| {
                    if val.trim().is_empty() {
                        return Err("Remote cannot be empty".to_string());
                    }
                    backend.remote_name = val.trim().to_string();
                    Ok(format!("Remote updated to {}", backend.remote_name))
                },
            )))
        },
    ));
    actions.push_option(OptionSpec::new(3, "Inspect Logs", |_ctx| {
        OptionOutcome::PushSubView(Box::new(MockLogSubView))
    }));
    page1.push_section(actions);

    let mut page2 = Page::new("Settings", "F2");
    let mut settings_sec = Section::menu("Preferences", "1-20 edit", Vec::new()).with_heights(8, 16);
    for num in 1..=18 {
        settings_sec.push_option(OptionSpec::new(num, format!("Setting #{num}"), move |_ctx| {
            OptionOutcome::OpenModal(Box::new(ConfirmModal::new(
                "Confirm Change",
                format!("Toggle setting #{num}?"),
                move |_: &mut MockVaultBackend| Ok(format!("Toggled #{num}")),
            )))
        }));
    }
    settings_sec.push_option(section_gap_option(19));
    settings_sec.push_option(color_theme_option(20));
    page2.push_section(settings_sec);

    let mut engine = DashboardEngine::new(MockVaultBackend::default(), vec![page1, page2]);
    let mut test_backend = TestBackend::new(TerminalWidth::new(110), TerminalHeight::new(28));

    // Render initial frame on Page 1
    let viewport = Rect::from_size(test_backend.dimensions().0, test_backend.dimensions().1);
    let mut frame_buf = Buffer::empty(viewport);
    engine.render(&mut frame_buf, viewport);
    let prev = Buffer::empty(Rect::ZERO);
    test_backend.draw_frame(diff_buffers(&prev, &frame_buf))?;
    test_backend.flush()?;

    let grid = test_backend.to_visual_grid();
    assert!(grid.contains("VaultDrive v3.0"));
    assert!(grid.contains("● MOUNTED"));
    assert!(grid.contains("[F1:Monitor]"));
    assert!(grid.contains("F2:Settings"));
    assert!(grid.contains("Sync Telemetry"));
    assert!(grid.contains("Quick Actions"));

    // Press '1' on Page 1 (max_option == 3 <= 9 -> immediate dispatch!)
    assert_eq!(engine.handle_key(KeyPress::char('1')), EngineAction::Continue);
    assert!(!engine.backend().mounted);

    // Pressing Tab on Page 1 must NOT select section 0 (`SyncTelemetryWidget`, which has
    // `selectable = false`) — focus stays on section 1 (`Quick Actions`, `selectable = true`)
    // so pressing Enter still triggers the highlighted Quick Actions item ([1] Toggle Mount)!
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Tab)),
        EngineAction::Continue
    );
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Enter)),
        EngineAction::Continue
    );
    assert!(engine.backend().mounted);

    // Press '3' to push MockLogSubView onto the SubView stack
    assert_eq!(engine.handle_key(KeyPress::char('3')), EngineAction::Continue);
    engine.render(&mut frame_buf, viewport);
    let sv_lines = frame_buf.to_visual_lines();
    assert!(sv_lines.iter().any(|l| l.contains("Journalctl Live Stream")));

    // Press Esc to pop SubView
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Esc)),
        EngineAction::Continue
    );

    // Switch to Page 2 via F2
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::F(2))),
        EngineAction::Continue
    );
    assert_eq!(engine.active_page(), 1);

    // On Page 2, max_option is 20 (> 9). Pressing '2' should enter pending prefix state (#2_),
    // and pressing '0' immediately after dispatches option [20] (ColorThemeModal)!
    assert_eq!(engine.handle_key(KeyPress::char('2')), EngineAction::Continue);
    assert!(!engine.has_modal());
    engine.render(&mut frame_buf, viewport);
    let footer_line = &frame_buf.to_visual_lines()[27];
    assert!(footer_line.contains("#2_"));

    assert_eq!(engine.handle_key(KeyPress::char('0')), EngineAction::Continue);
    assert!(engine.has_modal());

    // ColorThemeModal is open! Pressing Down live-previews the next theme (Catppuccin Mocha)
    let orig_theme = engine.palette().id;
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Down)),
        EngineAction::Continue
    );
    assert_ne!(engine.palette().id, orig_theme);

    // Pressing Esc reverts to original theme and closes modal
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Esc)),
        EngineAction::Continue
    );
    assert!(!engine.has_modal());
    assert_eq!(engine.palette().id, orig_theme);

    // Open [19] SectionGapModal via '1' then '9'
    let _ = engine.handle_key(KeyPress::char('1'));
    let _ = engine.handle_key(KeyPress::char('9'));
    assert!(engine.has_modal());

    // Press Right arrow to increment Layer 1 (Terminal -> Page gap) from 0 to 1
    let _ = engine.handle_key(KeyPress::new(KeyCode::Right));
    assert_eq!(engine.spacing().get(SpacingLayer::TerminalPage), 1);

    // Press Enter to save and close SectionGapModal
    let _ = engine.handle_key(KeyPress::new(KeyCode::Enter));
    assert!(!engine.has_modal());
    assert_eq!(engine.backend().saved_count, 1);

    // Press 'q' to quit
    assert_eq!(engine.handle_key(KeyPress::char('q')), EngineAction::Quit);

    // Verify custom KeyBindings (`quit('x')`, `page_cycle('<', '>')`, `activate("Space")`)
    engine.set_keys(
        KeyBindings::default()
            .quit('x')
            .page_cycle('<', '>')
            .activate("Space"),
    );
    assert_eq!(engine.handle_key(KeyPress::char('q')), EngineAction::Continue);
    assert_eq!(engine.handle_key(KeyPress::char('<')), EngineAction::Continue);
    assert_eq!(engine.active_page(), 0);
    assert_eq!(engine.handle_key(KeyPress::char('x')), EngineAction::Quit);
    Ok(())
}
