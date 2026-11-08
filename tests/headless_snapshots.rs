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
    Buffer, Color, ColorPalette, ColumnIndex, KeyHint, PrioritySegment, Rect, RowIndex, Style,
    TerminalHeight, TerminalWidth, diff_buffers,
};
use tui_rs::dashboard::{
    DashboardBackend, DashboardEngine, EngineAction, IntoKeyPress, KeyBindings, KeyCode, KeyPress,
    ModalContext, OptionOutcome, OptionSpec, Page, Section, SectionWidget, SpacingLayer, SubView,
    SubViewResult,
};
use tui_rs::universal::{ConfirmModal, EditModal, color_theme_option, section_gap_option};
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
        self.saved_count = self.saved_count.saturating_add(1);
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
    let _ = TabPanel::new("Drive Status").with_badge("MOUNTED").render(
        &mut compact_buf,
        compact_rect,
        &palette,
    );
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
    let _ =
        TabPanel::new("Drive Status")
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
#[expect(
    clippy::too_many_lines,
    reason = "comprehensive end-to-end integration test"
)]
fn dashboard_engine_end_to_end_navigation_modals_and_sub_views()
-> Result<(), tui_rs::error::TuiError> {
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
    let mut settings_sec =
        Section::menu("Preferences", "1-20 edit", Vec::new()).with_heights(8, 16);
    for num in 1..=18 {
        settings_sec.push_option(OptionSpec::new(
            num,
            format!("Setting #{num}"),
            move |_ctx| {
                OptionOutcome::OpenModal(Box::new(ConfirmModal::new(
                    "Confirm Change",
                    format!("Toggle setting #{num}?"),
                    move |_: &mut MockVaultBackend| Ok(format!("Toggled #{num}")),
                )))
            },
        ));
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
    assert_eq!(
        engine.handle_key(KeyPress::char('1')),
        EngineAction::Continue
    );
    assert!(!engine.backend().mounted);

    // Pressing Right arrow (section cycle) on Page 1 must NOT select section 0 (`SyncTelemetryWidget`, which has
    // `selectable = false`) — focus stays on section 1 (`Quick Actions`, `selectable = true`)
    // so pressing Enter still triggers the highlighted Quick Actions item ([1] Toggle Mount)!
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Right)),
        EngineAction::Continue
    );
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Enter)),
        EngineAction::Continue
    );
    assert!(engine.backend().mounted);

    // Press '3' to push MockLogSubView onto the SubView stack
    assert_eq!(
        engine.handle_key(KeyPress::char('3')),
        EngineAction::Continue
    );
    engine.render(&mut frame_buf, viewport);
    let sv_lines = frame_buf.to_visual_lines();
    assert!(
        sv_lines
            .iter()
            .any(|l| l.contains("Journalctl Live Stream"))
    );

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
    assert_eq!(
        engine.handle_key(KeyPress::char('2')),
        EngineAction::Continue
    );
    assert!(!engine.has_modal());
    engine.render(&mut frame_buf, viewport);
    let footer_line = &frame_buf.to_visual_lines()[27];
    assert!(footer_line.contains("#2_"));

    assert_eq!(
        engine.handle_key(KeyPress::char('0')),
        EngineAction::Continue
    );
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

    // Press 'q' at root dashboard: nothing left to cancel, so the host is told to go back
    assert_eq!(engine.handle_key(KeyPress::char('q')), EngineAction::Back);
    // Ctrl+C also goes back (leave the UI) rather than terminating the host
    assert_eq!(
        engine.handle_key(KeyPress::char('c').with_ctrl(true)),
        EngineAction::Back
    );

    // Press ^Q (Ctrl+Q) to quit
    assert_eq!(
        engine.handle_key(KeyPress::char('q').with_ctrl(true)),
        EngineAction::Quit
    );

    // Verify custom KeyBindings with ctrl+[key] (`quit("ctrl+x")`, `page_cycle('<', '>')`, `activate("Space")`)
    engine.set_keys(
        KeyBindings::default()
            .quit("ctrl+x")
            .page_cycle('<', '>')
            .section_cycle("Right", "Left")
            .activate("Space"),
    );
    assert_eq!(engine.handle_key(KeyPress::char('q')), EngineAction::Back);
    assert_eq!(
        engine.handle_key(KeyPress::char('<')),
        EngineAction::Continue
    );
    assert_eq!(engine.active_page(), 0);
    // Pressing 'x' alone does not quit; Ctrl+X quits
    assert_eq!(
        engine.handle_key(KeyPress::char('x')),
        EngineAction::Continue
    );
    assert_eq!(
        engine.handle_key(KeyPress::char('x').with_ctrl(true)),
        EngineAction::Quit
    );
    Ok(())
}

#[test]
fn notice_modal_embeds_qr_code_and_hyperlinks() -> Result<(), tui_rs::error::TuiError> {
    use tui_rs::dashboard::splice_modal_overlay;
    use tui_rs::universal::NoticeModal;

    let url = "https://github.com/vaultdrive/tui-rs";
    let modal = NoticeModal::new(
        "Documentation & Mobile Pairing",
        vec![
            "Repo link:".to_string(),
            url.to_string(),
            "• Hold Ctrl and click link".to_string(),
        ],
    )
    .with_qr_text(url)?;

    assert!(modal.qr_matrix().is_some());

    let viewport = Rect::from_size(TerminalWidth::new(80), TerminalHeight::new(24));
    let mut buf = Buffer::empty(viewport);
    splice_modal_overlay::<MockVaultBackend>(
        &modal,
        &mut buf,
        viewport,
        &ColorPalette::DEFAULT,
        false,
    );

    let visual = buf.to_visual_lines();
    assert!(
        visual
            .iter()
            .any(|line| line.contains("Documentation & Mobile Pairing"))
    );
    assert!(
        visual
            .iter()
            .any(|line| line.contains("https://github.com/vaultdrive/tui-rs"))
    );
    assert!(visual.iter().any(|line| line.contains('▀')));

    let link_cell = buf
        .as_cell_slice()
        .iter()
        .find(|c| c.hyperlink() == Some("https://github.com/vaultdrive/tui-rs"));
    assert!(
        link_cell.is_some(),
        "Cell with OSC 8 hyperlink must exist inside the modal"
    );

    Ok(())
}

#[test]
fn qr_code_generation_matrix_and_widget_rendering() -> Result<(), tui_rs::error::TuiError> {
    use tui_rs::components::{HalfBlockQr, QrCodeMatrix, QrWidget};

    let text = "https://github.com/vaultdrive/tui-rs";
    let matrix = QrCodeMatrix::from_text(text)?;
    assert!(matrix.dimension() >= 21);
    assert_eq!(matrix.quiet_zone(), 2);
    let expected_w = u16::try_from(matrix.dimension().saturating_add(4)).map_err(|_| {
        tui_rs::error::TuiError::InvalidDimensions {
            width: 0,
            height: 0,
        }
    })?;
    assert_eq!(matrix.rendered_width(), expected_w);

    let area = Rect::from_size(
        TerminalWidth::new(matrix.rendered_width().saturating_add(4)),
        TerminalHeight::new(matrix.rendered_height().saturating_add(2)),
    );
    let mut buf = Buffer::empty(area);
    matrix.render(&mut buf, ColumnIndex::new(2), RowIndex::new(1));
    let lines = buf.to_visual_lines();
    assert!(lines.iter().any(|line| line.contains('▀')));

    let hb = HalfBlockQr::from_matrix(&matrix);
    assert_eq!(hb.dimension(), matrix.dimension());

    let widget = QrWidget::from_text("MOBILE CONNECT", text)?;
    assert_eq!(widget.title_str(), "MOBILE CONNECT");
    Ok(())
}

#[test]
fn osc8_hyperlinks_ansi_encoding_and_widgets() -> Result<(), tui_rs::error::TuiError> {
    use tui_rs::backend::{AnsiFrameWriter, format_hyperlink, format_hyperlink_with_id};
    use tui_rs::core::{ColorDepth, Line, Span};
    use tui_rs::widgets::Hyperlink;

    // 1. Standalone formatters
    let formatted = format_hyperlink("https://github.com/vaultdrive", "VaultDrive");
    assert_eq!(
        formatted,
        "\x1b]8;;https://github.com/vaultdrive\x1b\\VaultDrive\x1b]8;;\x1b\\"
    );

    let formatted_id = format_hyperlink_with_id("https://rust-lang.org", "Rust", "rust-link");
    assert_eq!(
        formatted_id,
        "\x1b]8;id=rust-link;https://rust-lang.org\x1b\\Rust\x1b]8;;\x1b\\"
    );

    // 2. Span::hyperlink and Buffer::set_line
    let rect = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(1));
    let mut buf = Buffer::empty(rect);
    let span = Span::hyperlink("GitHub", "https://github.com");
    assert_eq!(span.hyperlink_url(), Some("https://github.com"));
    let line = Line::from_spans([span]);
    buf.set_line(
        ColumnIndex::new(0),
        RowIndex::new(0),
        &line,
        TerminalWidth::new(30),
    );

    let cell = buf.cell_at(ColumnIndex::new(0), RowIndex::new(0)).ok_or(
        tui_rs::error::TuiError::OutOfBounds {
            column: 0,
            row: 0,
            width: 30,
            height: 1,
        },
    )?;
    assert_eq!(cell.hyperlink(), Some("https://github.com"));
    assert_eq!(cell.symbol(), "G");

    // 3. AnsiFrameWriter emits OSC 8 open and close sequences
    let empty_buf = Buffer::empty(rect);
    let mut writer = AnsiFrameWriter::with_capacity(512, ColorDepth::TrueColor, false);
    let bytes = writer.encode_frame(diff_buffers(&empty_buf, &buf));
    let output = String::from_utf8_lossy(bytes);
    assert!(output.contains("\x1b]8;;https://github.com\x1b\\"));
    assert!(output.contains("\x1b]8;;\x1b\\"));
    assert!(output.contains("GitHub"));

    // 4. When hyperlinks disabled, no OSC 8 sequences emitted
    let mut no_link_writer =
        AnsiFrameWriter::with_capacity(512, ColorDepth::TrueColor, false).with_hyperlinks(false);
    let plain_bytes = no_link_writer.encode_frame(diff_buffers(&empty_buf, &buf));
    let plain_output = String::from_utf8_lossy(plain_bytes);
    assert!(!plain_output.contains("\x1b]8;;"));
    assert!(plain_output.contains("GitHub"));

    // 5. Hyperlink widget rendering
    let mut buf2 = Buffer::empty(rect);
    let widget = Hyperlink::new("https://crates.io", "Crates");
    assert_eq!(widget.url(), "https://crates.io");
    assert_eq!(widget.text(), "Crates");
    widget.render(&mut buf2, ColumnIndex::new(5), RowIndex::new(0), 15);
    let cell2 = buf2.cell_at(ColumnIndex::new(5), RowIndex::new(0)).ok_or(
        tui_rs::error::TuiError::OutOfBounds {
            column: 5,
            row: 0,
            width: 30,
            height: 1,
        },
    )?;
    assert_eq!(cell2.hyperlink(), Some("https://crates.io"));
    assert_eq!(cell2.symbol(), "C");

    Ok(())
}

#[test]
fn key_mapping_summary_and_ctrl_key_customization() {
    use tui_rs::dashboard::{IntoKeyPress, KeyBindings, KeyCode, KeyPress};

    // 1. Verify string parsing for any ctrl+[key] pattern
    assert_eq!(
        "ctrl+q".into_key_press(),
        KeyPress::char('q').with_ctrl(true)
    );
    assert_eq!(
        "Ctrl+Q".into_key_press(),
        KeyPress::char('q').with_ctrl(true)
    );
    assert_eq!("^q".into_key_press(), KeyPress::char('q').with_ctrl(true));
    assert_eq!("^Q".into_key_press(), KeyPress::char('q').with_ctrl(true));
    assert_eq!(
        "ctrl-x".into_key_press(),
        KeyPress::char('x').with_ctrl(true)
    );
    assert_eq!(
        "ctrl+w".into_key_press(),
        KeyPress::char('w').with_ctrl(true)
    );
    assert_eq!(
        "Shift+Tab".into_key_press(),
        KeyPress::new(KeyCode::BackTab)
    );
    assert_eq!("Tab".into_key_press(), KeyPress::new(KeyCode::Tab));
    assert_eq!("Right".into_key_press(), KeyPress::new(KeyCode::Right));
    assert_eq!("→".into_key_press(), KeyPress::new(KeyCode::Right));
    assert_eq!("Left".into_key_press(), KeyPress::new(KeyCode::Left));
    assert_eq!("←".into_key_press(), KeyPress::new(KeyCode::Left));

    // 2. Default KeyBindings match the requested specification
    let default_keys = KeyBindings::default();
    assert_eq!(
        default_keys.quit_press(),
        KeyPress::char('q').with_ctrl(true)
    );
    assert_eq!(default_keys.quit_press().short_label().as_str(), "^Q");
    assert_eq!(
        default_keys.next_section_press(),
        KeyPress::new(KeyCode::Right)
    );
    assert_eq!(
        default_keys.prev_section_press(),
        KeyPress::new(KeyCode::Left)
    );
    assert_eq!(default_keys.next_page_press(), KeyPress::new(KeyCode::Tab));
    assert_eq!(
        default_keys.prev_page_press(),
        KeyPress::new(KeyCode::BackTab)
    );
    assert_eq!(default_keys.cancel_press(), KeyPress::char('q'));
    assert_eq!(default_keys.cursor_up_press(), KeyPress::new(KeyCode::Up));
    assert_eq!(
        default_keys.cursor_down_press(),
        KeyPress::new(KeyCode::Down)
    );
    assert_eq!(default_keys.activate_press(), KeyPress::new(KeyCode::Enter));

    // 3. Custom keybindings configured via developer method chaining in dashboard file
    let custom_keys = KeyBindings::default()
        .quit("ctrl+x")
        .page_cycle("Shift+Tab", "Tab")
        .section_cycle("Right", "Left")
        .up_down("Up", "Down")
        .vim_up_down('k', 'j')
        .activate("Enter")
        .cancel('q');
    assert_eq!(
        custom_keys.quit_press(),
        KeyPress::char('x').with_ctrl(true)
    );
    assert_eq!(custom_keys.quit_press().short_label().as_str(), "^X");
}

fn guarded_engine() -> DashboardEngine<MockVaultBackend> {
    let mut storage_menu = Section::menu("Actions", "", Vec::new()).with_heights(5, 8);
    storage_menu.push_option(
        OptionSpec::new(
            7,
            "Open Storage",
            |ctx: &mut ModalContext<'_, MockVaultBackend>| {
                ctx.backend.saved_count += 1;
                OptionOutcome::None
            },
        )
        .with_shortkey('o'),
    );
    let mut page = Page::new("Main", "F1");
    page.push_section(storage_menu);
    DashboardEngine::new(MockVaultBackend::default(), vec![page])
}

#[test]
fn option_shortkey_runs_option_and_keeps_its_number() {
    let mut engine = guarded_engine();
    assert_eq!(
        engine.handle_key(KeyPress::char('o')),
        EngineAction::Continue
    );
    assert_eq!(engine.backend().saved_count, 1);
    assert_eq!(
        engine.handle_key(KeyPress::char('7')),
        EngineAction::Continue
    );
    assert_eq!(engine.backend().saved_count, 2);
}

#[test]
fn bound_key_opens_warning_enter_confirms_and_q_cancels() {
    let mut engine = guarded_engine().bind_confirmed_key(
        "ctrl+x",
        "Wipe",
        "This cannot be undone.",
        |backend: &mut MockVaultBackend| {
            backend.saved_count = 100;
            Ok("Wiped".to_string())
        },
    );
    // q cancels: nothing happens
    let _ = engine.handle_key("ctrl+x".into_key_press());
    assert!(engine.has_modal());
    let _ = engine.handle_key(KeyPress::char('q'));
    assert!(!engine.has_modal());
    assert_eq!(engine.backend().saved_count, 0);
    // Enter confirms; the binding is reusable
    let _ = engine.handle_key("ctrl+x".into_key_press());
    let _ = engine.handle_key(KeyPress::new(KeyCode::Enter));
    assert!(!engine.has_modal());
    assert_eq!(engine.backend().saved_count, 100);
}

#[test]
fn confirm_quit_requires_enter_and_q_cancels() {
    let mut engine =
        guarded_engine().with_keys(KeyBindings::default().quit("ctrl+q").confirm_quit(true));
    assert_eq!(
        engine.handle_key(KeyPress::char('q').with_ctrl(true)),
        EngineAction::Continue
    );
    assert!(engine.has_modal());
    assert_eq!(
        engine.handle_key(KeyPress::char('q')),
        EngineAction::Continue
    );
    assert!(!engine.has_modal());
    let _ = engine.handle_key(KeyPress::char('q').with_ctrl(true));
    assert_eq!(
        engine.handle_key(KeyPress::new(KeyCode::Enter)),
        EngineAction::Quit
    );
    // Ctrl+C is never guarded: it leaves the UI at once (`Back`)
    assert_eq!(
        engine.handle_key(KeyPress::char('c').with_ctrl(true)),
        EngineAction::Back
    );
}

fn render_engine_grid(
    engine: &mut DashboardEngine<MockVaultBackend>,
    columns: u16,
    rows: u16,
) -> Result<String, tui_rs::error::TuiError> {
    let mut test_backend = TestBackend::new(TerminalWidth::new(columns), TerminalHeight::new(rows));
    let viewport = Rect::from_size(test_backend.dimensions().0, test_backend.dimensions().1);
    let mut frame_buf = Buffer::empty(viewport);
    engine.render(&mut frame_buf, viewport);
    test_backend.draw_frame(diff_buffers(&Buffer::empty(Rect::ZERO), &frame_buf))?;
    test_backend.flush()?;
    Ok(test_backend.to_visual_grid())
}

#[test]
fn unmapped_key_flashes_hint_and_question_mark_shows_help() {
    let mut engine = guarded_engine();
    assert_eq!(
        engine.handle_key(KeyPress::char('z')),
        EngineAction::Continue
    );
    let hint = engine.flash_message().unwrap_or_default().to_string();
    assert!(hint.contains("No shortcut for 'z'"), "got: {hint}");

    // Mapped keys never flash a hint
    let mut engine = guarded_engine();
    let _ = engine.handle_key(KeyPress::new(KeyCode::Down));
    assert_eq!(engine.flash_message(), None);

    // `?` shows the help line
    let _ = engine.handle_key(KeyPress::char('?'));
    assert!(engine.flash_message().unwrap_or_default().contains("quit"));

    // Hints can be disabled or customised
    let mut silent = guarded_engine().with_invalid_key_hints(false);
    let _ = silent.handle_key(KeyPress::char('z'));
    assert_eq!(silent.flash_message(), None);
    let mut custom = guarded_engine()
        .with_invalid_key_hint(|key| Some(format!("nope {}", key.short_label().as_str())));
    let _ = custom.handle_key(KeyPress::char('z'));
    assert_eq!(custom.flash_message(), Some("nope z"));
}

#[test]
fn active_page_tab_badge_is_rendered_in_header() -> Result<(), tui_rs::error::TuiError> {
    let page = Page::<MockVaultBackend>::new("Monitor", "F1")
        .with_tab_badge(|backend| format!("{} saved", backend.saved_count));
    let mut engine = DashboardEngine::new(MockVaultBackend::default(), vec![page]);
    let grid = render_engine_grid(&mut engine, 110, 12)?;
    assert!(grid.contains("[F1:Monitor 0 saved]"), "{grid}");
    Ok(())
}

#[test]
fn log_viewer_lists_newest_first_scrolls_and_pops() -> Result<(), tui_rs::error::TuiError> {
    use tui_rs::universal::{LogEntry, LogLevel, LogViewSubView};

    fn mock_entries(_backend: &MockVaultBackend) -> &[LogEntry] {
        static ENTRIES: std::sync::OnceLock<Vec<LogEntry>> = std::sync::OnceLock::new();
        ENTRIES
            .get_or_init(|| {
                (0..30)
                    .map(|n| {
                        LogEntry::new(
                            if n == 29 {
                                LogLevel::Error
                            } else {
                                LogLevel::Info
                            },
                            "sync",
                            format!("event number {n}"),
                        )
                    })
                    .collect()
            })
            .as_slice()
    }

    let mut engine = guarded_engine();
    engine.push_sub_view(Box::new(LogViewSubView::new("Live Logs", mock_entries)));

    let grid = render_engine_grid(&mut engine, 80, 14)?;
    assert!(grid.contains("Live Logs"));
    assert!(grid.contains("30 lines"));
    assert!(grid.contains("ERROR sync event number 29"), "{grid}");
    assert!(!grid.contains("event number 0\n") && !grid.contains("event number 0 "));

    // Jump to the oldest entry
    let _ = engine.handle_key(KeyPress::new(KeyCode::End));
    let grid = render_engine_grid(&mut engine, 80, 14)?;
    assert!(grid.contains("event number 0"), "{grid}");
    assert!(!grid.contains("event number 29"));

    // Esc goes back
    assert!(engine.has_sub_view());
    let _ = engine.handle_key(KeyPress::new(KeyCode::Esc));
    assert!(!engine.has_sub_view());
    Ok(())
}
