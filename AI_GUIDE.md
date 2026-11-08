# `dashboard-tui` — Exhaustive AI Agent Architecture & Integration Manual

> **TARGET AUDIENCE: AI Coding Agents & Autonomous Systems Architects**
>
> This document is the single source of truth for AI agents integrating `dashboard-tui` into host Rust applications (such as `vaultdrive-linux` or any system daemon/CLI dashboard) or extending `dashboard-tui` itself.
>
> - **Before modifying `dashboard-tui` internals**, read [`RULES.md`](./RULES.md).
> - **Before building a dashboard in a host project**, read **Sections 1–6** of this guide.
> - **Reference working showcase**: [`examples/vaultdrive_dashboard.rs`](./examples/vaultdrive_dashboard.rs) and [`tests/headless_snapshots.rs`](./tests/headless_snapshots.rs).

---

## 1. Fast Navigation Matrix ("Where Should the AI Agent Look?")

Use this table to jump directly to the exact file in `src/` for any task without scanning the entire repository:

| If You Need To... | Read This File First | Key Types / Functions to Import (`use dashboard_tui::prelude::*;`) |
| :--- | :--- | :--- |
| **Wire a complete multi-page dashboard in `tui/dashboard.rs`** | [`src/dashboard/registry.rs`](./src/dashboard/registry.rs) | `AddonRegistry`, `SectionLayoutMode`, `DashboardEngine` |
| **Connect host app state to the dashboard** | [`src/dashboard/backend_trait.rs`](./src/dashboard/backend_trait.rs) | `DashboardBackend`, `PrioritySegment`, `Color`, `Style` |
| **Build a custom live telemetry panel (`tui/addons/*.rs`)** | [`src/dashboard/section.rs`](./src/dashboard/section.rs) | `SectionWidget`, `Section`, `Buffer`, `Rect`, `ColorPalette` |
| **Add numbered menu options (`[1]`..`[99]`) in `tui/addons/*.rs`** | [`src/dashboard/section.rs`](./src/dashboard/section.rs) | `OptionSpec`, `OptionOutcome`, `ModalContext` |
| **Open a built-in Edit / Confirm / Typed / Notice dialog** | [`src/universal/mod.rs`](./src/universal/mod.rs) | `EditModal`, `ConfirmModal`, `TypedModal`, `NoticeModal` |
| **Add built-in Spacing Gap (`[19]`) & Color Theme (`[20]`) options** | [`src/universal/section_gap_modal.rs`](./src/universal/section_gap_modal.rs), [`src/universal/color_theme_modal.rs`](./src/universal/color_theme_modal.rs) | `section_gap_option(19)`, `color_theme_option(20)` |
| **Create a full-screen drill-down view (e.g., Live Log Inspector)** | [`src/dashboard/sub_view.rs`](./src/dashboard/sub_view.rs) | `SubView`, `SubViewResult`, `KeyPress`, `KeyCode`, `KeyHint` |
| **Create a custom centered modal overlay** | [`src/dashboard/modal.rs`](./src/dashboard/modal.rs) | `Modal`, `ModalResult`, `ModalContext` |
| **Render an Upload/Download mirrored Braille waveform** | [`src/components/dual_waveform.rs`](./src/components/dual_waveform.rs), [`src/components/wave_scaler.rs`](./src/components/wave_scaler.rs) | `DualWaveform`, `SingleWaveform`, `WaveScaler`, `WaveScalerConfig`, `GraphSymbolMode` |
| **Store & resample time-windowed telemetry samples** | [`src/components/time_series.rs`](./src/components/time_series.rs) | `TimeSeries`, `TimedSample`, `resample_slice_scaled_into`, `resample_slice_into` |
| **Format bytes (`1.25 GB`) or rates (`▲ 512 KB/s`) with 0 heap allocs** | [`src/components/formatters.rs`](./src/components/formatters.rs) | `format_bytes`, `format_rate`, `format_rate_short`, `spinner_frame`, `StackStr` |
| **Render a gradient progress bar, slider, or half-block QR code** | [`src/components/meter.rs`](./src/components/meter.rs), [`src/components/slider.rs`](./src/components/slider.rs), [`src/components/qr_code.rs`](./src/components/qr_code.rs) | `GradientMeter`, `SliderBar`, `HalfBlockQr` |
| **Generate QR codes from text/URLs & embed in dashboards** | [`src/components/qr_code.rs`](./src/components/qr_code.rs), [`src/components/qr_widget.rs`](./src/components/qr_widget.rs) | `QrCodeMatrix`, `QrErrorCorrection`, `QrWidget`, `HalfBlockQr` |
| **Embed clickable terminal web-links (OSC 8 hyperlinks)** | [`src/widgets/hyperlink.rs`](./src/widgets/hyperlink.rs), [`src/backend/ansi_writer.rs`](./src/backend/ansi_writer.rs), [`src/core/text.rs`](./src/core/text.rs) | `Hyperlink`, `Span::hyperlink`, `format_hyperlink`, `format_hyperlink_with_id` |
| **Render a `btop`-style notched border box or data table** | [`src/widgets/panel.rs`](./src/widgets/panel.rs), [`src/widgets/weighted_table.rs`](./src/widgets/weighted_table.rs) | `TabPanel`, `WeightedTable`, `TableColumn`, `TextAlignment` |
| **Render a list, table, paragraph, tabs, gauge, sparkline, bar/line chart, canvas or calendar** | [`src/widgets/`](./src/widgets/) (`list.rs`, `table.rs`, `paragraph.rs`, `tabs.rs`, `gauge.rs`, `sparkline.rs`, `bar_chart.rs`, `chart.rs`, `canvas.rs`, `calendar.rs`, `clear.rs`), [`src/core/widget.rs`](./src/core/widget.rs) | `Widget`, `StatefulWidget`, `List`/`ListState`, `Table`/`TableState`, `Paragraph`, `Tabs`, `Gauge`, `LineGauge`, `Sparkline`, `BarChart`, `Chart`, `Canvas`, `Calendar`, `Clear` |
| **Render a scrollable numbered menu, choice row, or text input** | [`src/widgets/menu_list.rs`](./src/widgets/menu_list.rs), [`src/widgets/choice_row.rs`](./src/widgets/choice_row.rs), [`src/widgets/text_input.rs`](./src/widgets/text_input.rs) | `MenuList`, `MenuItem`, `ChoiceRow`, `TextInput`, `TextInputState`, `SubCellScrollbar` |
| **Write strings/cells to the 2D screen buffer or split rects** | [`src/core/buffer.rs`](./src/core/buffer.rs), [`src/core/rect.rs`](./src/core/rect.rs), [`src/core/layout.rs`](./src/core/layout.rs) | `Buffer`, `Rect`, `ColumnIndex`, `RowIndex`, `TerminalWidth`, `TerminalHeight`, `Layout`, `Constraint` |
| **Write deterministic headless tests without a real terminal** | [`src/backend/test_backend.rs`](./src/backend/test_backend.rs), [`tests/headless_snapshots.rs`](./tests/headless_snapshots.rs) | `TestBackend`, `Buffer::to_visual_lines()`, `KeyPress`, `KeyCode` |

---

## 2. Complete Repository Sitemap (`src/`)

`dashboard-tui` is organized into **7 strictly layered modules** (`61` source files total, zero cyclic dependencies, `#![forbid(unsafe_code)]`, zero `.unwrap()`/`.expect()` in production paths):

```text
/home/cee/Documents/dashboard-tui/
├── Cargo.toml                         # Crate manifest (Cargo lints, dependencies)
├── RULES.md                           # Mandatory engineering & API design laws
├── AI_GUIDE.md                        # This AI integration & architecture manual
├── examples/
│   └── vaultdrive_dashboard.rs        # Complete 3-page VaultDrive reference showcase
├── tests/
│   ├── headless_snapshots.rs          # Headless visual snapshot & E2E interaction tests
│   └── proptest_invariants.rs         # Property-based fuzz tests (layout, graphemes, scaler)
└── src/
    ├── lib.rs                         # Crate root & module exports
    ├── error.rs                       # TuiError#[non_exhaustive] domain error enum
    ├── prelude.rs                     # Single-import ergonomic prelude (`use dashboard_tui::prelude::*;`)
    │
    ├── core/                          # LAYER 1: Zero-Allocation Primitives, Geometry, Buffer, Themes
    │   ├── mod.rs
    │   ├── coordinates.rs             # ColumnIndex, RowIndex, DisplayWidth newtypes
    │   ├── rect.rs                    # TerminalWidth, TerminalHeight, Spacing4D, Rect
    │   ├── color.rs                   # Color (Ansi16, Indexed256, Rgb), ColorDepth, downgrade()
    │   ├── palette.rs                 # ColorPalette (DEFAULT, CATPPUCCIN_MOCHA, TOKYO_NIGHT, GRUVBOX_DARK, NORD, TTY), GradientLut
    │   ├── style.rs                   # Style, Modifiers bitflags (BOLD, DIM, ITALIC, UNDERLINED, REVERSED, etc.)
    │   ├── grapheme.rs                # CompactSymbol (23-byte inline SSO), display_width, truncate_to_width, pad_or_truncate
    │   ├── stack_str.rs               # StackStr<const N: usize> zero-heap stack string buffer
    │   ├── cell.rs                    # Cell (symbol + style + wide-char continuation flag)
    │   ├── buffer.rs                  # Buffer (2D row-major grid, set_cell, set_string, fill_rect, splice_into, to_visual_lines)
    │   ├── diff.rs                    # CellDiff, diff_buffers(previous, current) -> DiffIter
    │   ├── text.rs                    # Span<'a>, Line<'a>, TextAlignment (Left, Center, Right)
    │   ├── layout.rs                  # Constraint (Fixed, Min, Max, Percentage, Ratio, Fill), LayoutDirection, Layout, split_equal_spans
    │   └── priority_strip.rs          # PrioritySegment, KeyHint, render_priority_strip, render_key_hints
    │
    ├── backend/                       # LAYER 2: ANSI/DEC Escape Encoder, Crossterm & Headless Test Backends
    │   ├── mod.rs                     # Backend trait
    │   ├── ansi_writer.rs             # AnsiFrameWriter (single-flush CSI/SGR/DEC 2026/OSC 9;4 encoder)
    │   ├── tty_detector.rs            # TerminalCapabilities (Linux console TTY & color depth auto-detection)
    │   ├── crossterm_backend.rs       # CrosstermBackend<W: Write> production terminal backend
    │   └── test_backend.rs            # TestBackend deterministic in-memory headless backend
    │
    ├── terminal/                      # LAYER 3: RAII Session Guard & Composable Panic Hook
    │   ├── mod.rs
    │   ├── panic_hook.rs              # install_panic_restoration_hook(), restore_terminal_emergency()
    │   └── session_guard.rs           # TerminalSessionConfig, TerminalSessionGuard<B> (RAII Drop + double-buffered draw)
    │
    ├── components/                    # LAYER 4: Telemetry Visualizations (Braille Waveforms, Scaler, Meters, QR)
    │   ├── mod.rs
    │   ├── graph_symbols.rs           # GraphSymbolMode (Braille, Block, Tty), 5x5 LUTs (graph_up_cell, graph_down_cell)
    │   ├── wave_scaler.rs             # WaveScaler, WaveScalerConfig, nice_ceil, round_scale (deadband + 5-sample decay)
    │   ├── time_series.rs             # TimeSeries<N>, TimedSample, resample_slice_scaled_into, resample_slice_into
    │   ├── single_waveform.rs         # SingleWaveform (zero-alloc up/down waveform, full-width auto-scaling)
    │   ├── dual_waveform.rs           # DualWaveform (mirrored top/bottom waveform + ▲/▼/[≤Scale] center axis)
    │   ├── meter.rs                   # GradientMeter ([■■■■□□□□] 75% smooth gradient bar)
    │   ├── slider.rs                  # SliderBar (interactive discrete/continuous value bar)
    │   ├── qr_code.rs                 # QrCodeMatrix, QrErrorCorrection, HalfBlockQr (QR matrix generation & 2x half-block packing)
    │   ├── qr_widget.rs               # QrWidget (centered QR display widget implementing SectionWidget)
    │   └── formatters.rs              # format_bytes, format_rate, format_rate_short, spinner_frame (zero heap alloc)
    │
    ├── widgets/                       # LAYER 5: Structural UI Widgets (Notched Panels, Menus, Tables, Inputs, Links)
    │   ├── mod.rs
    │   ├── panel.rs                   # TabPanel (btop-style ╭─┐ Title ┌─╮ with 4-step width degradation & TTY fallback)
    │   ├── divider.rs                 # LabeledDivider (── Label ───────)
    │   ├── hyperlink.rs               # Hyperlink (OSC 8 clickable terminal web-link widget & Span conversion)
    │   ├── scrollbar.rs               # VerticalScrollState, SubCellScrollbar (8x vertical Braille/block thumb)
    │   ├── menu_list.rs               # MenuItem, MenuList (1-col & auto 2-col at width >= 100, 4D option_gap, scrollbar)
    │   ├── weighted_table.rs          # TableColumn, WeightedTable (proportional column solver with min_width)
    │   ├── text_input.rs              # TextInputState, TextInput (grapheme-aware cursor, horizontal pan, masked password mode)
    │   └── choice_row.rs              # ChoiceRow (horizontal pill selector [writes]  full  off)
    │
    ├── dashboard/                     # LAYER 6: Tier 1 Portable Dashboard Engine & Declarative Add-On Registry
    │   ├── mod.rs
    │   ├── backend_trait.rs           # DashboardBackend trait (the host application contract)
    │   ├── spacing.rs                 # SpacingLayer (4 layers), SpacingConfig
    │   ├── key_router.rs              # KeyCode, KeyPress, EventState, EngineAction
    │   ├── navigation.rs              # PrefixJumpState (1..99 multi-digit shortcut state machine with 450ms window)
    │   ├── modal.rs                   # Modal<B>, ModalContext, ModalResult, splice_modal_overlay
    │   ├── sub_view.rs                # SubView<B>, SubViewResult (full-area drill-down view stack)
    │   ├── section.rs                 # SectionRole, SectionVisibility, SectionFocusStyle, SectionWidget<B>, OptionSpec<B>, OptionOutcome<B>, Section<B>
    │   ├── page.rs                    # SectionLayoutMode, ResolvedSectionRect, Page<B>
    │   ├── chrome.rs                  # render_header_bar, render_footer_bar (priority-pruned top & bottom bars)
    │   ├── registry.rs                # AddonRegistry<B>, AddonPageEntry, AddonSectionEntry, AddonOptionEntry, DashboardEngine::run_interactive
    │   └── engine.rs                  # DashboardEngine<B> (state machine, key routing, layout, modal/subview splicing)
    │
    └── universal/                     # LAYER 7: Tier 2 Universal Add-Ons & Reusable Modal Implementations
        ├── mod.rs
        ├── section_gap_modal.rs       # SectionGapModal & section_gap_option(num) (live 4-layer spacing editor)
        ├── color_theme_modal.rs       # ColorThemeModal & color_theme_option(num) (live 5-palette theme switcher)
        ├── edit_modal.rs              # EditModal<B> (text input modal with validation callback)
        ├── confirm_modal.rs           # ConfirmModal<B> (Yes/No confirmation modal)
        ├── typed_modal.rs             # TypedModal<B> (destructive action modal requiring exact phrase match)
        ├── settings_modal.rs          # SettingsModal + SettingField (data-driven multi-field form, live preview, revert on cancel)
        ├── status_panels.rs           # StatusPanel (meter/text/rate rows) + ThroughputPanel (live braille waveform)
        ├── log_view.rs                # LogViewSubView + LogEntry/LogLevel (scrollable, newest-first log page)
        └── notice_modal.rs            # NoticeModal (multi-line informational alert modal)
```

---

## 3. Canonical Host Project Structure (`src/tui/dashboard.rs` + `src/tui/addons/`)

When building a dashboard for a project (such as `vaultdrive-linux`), **ALWAYS** structure the host project's TUI module into:
1. **`src/tui/dashboard.rs`**: A **minimal declarative blueprint** (~30–50 lines) that registers pages, sections, and options from `src/tui/addons/` using [`AddonRegistry`](./src/dashboard/registry.rs) and launches the session via `.build_engine(app).run_interactive(...)`.
2. **`src/tui/addons/`**: A directory of modular add-on files where **all** widget rendering (`SectionWidget`), option actions (`OptionSpec`), custom modals (`Modal`), and drill-down screens (`SubView`) are implemented.

```text
host-project/
└── src/
    ├── main.rs
    ├── app.rs                    # Host domain state & Implementing `DashboardBackend`
    └── tui/
        ├── mod.rs                # `pub mod addons; pub mod dashboard;`
        ├── dashboard.rs          # Minimal declarative manifest (Pages + Sections + Options)
        └── addons/
            ├── mod.rs            # Exports all add-on modules
            ├── status_panel.rs   # Display Widget: `impl SectionWidget<App>`
            ├── wave_panel.rs     # Display Widget: `DualWaveform` + `WaveScaler`
            ├── quick_actions.rs  # Menu Options: `[1]`..`[5]` returning `OptionSpec<App>`
            ├── settings_menu.rs  # Menu Options: `[1]`..`[18]` returning `OptionSpec<App>`
            └── log_inspector.rs  # Drill-Down SubView: `impl SubView<App>`
```

### Why This Pattern Is Mandatory
- **Zero Sprawl in `dashboard.rs`**: Anyone (human or AI) reading `src/tui/dashboard.rs` immediately sees the entire page/section/option hierarchy at a glance.
- **Independent Add-On Modules**: Adding or modifying option `[7]` or a telemetry panel only touches a single small file inside `src/tui/addons/`.
- **Zero Engine Edits**: `dashboard-tui` handles all focus borders, `F1`..`F12` / `[` `]` page tabs, `Tab` section cycling, `1`..`99` multi-digit key jumping, 2-column wide menu layout (`width >= 100`), scrollbars, modal backdrop dimming, and terminal resize adaptation automatically.

---

## 4. Step-by-Step Implementation Blueprints for AI Agents

### Step 1: Implement `DashboardBackend` on the Host Application State

File reference: [`src/dashboard/backend_trait.rs`](./src/dashboard/backend_trait.rs)

```rust
use dashboard_tui::prelude::*;

pub struct VaultDriveApp {
    pub mounted: bool,
    pub syncing: bool,
    pub remote_path: String,
    pub mount_point: String,
    pub cache_used_bytes: u64,
    pub cache_max_bytes: u64,
    pub up_samples: Vec<f64>,
    pub down_samples: Vec<f64>,
    pub scaler: WaveScaler,
}

impl DashboardBackend for VaultDriveApp {
    /// Displayed on the far left of the top header bar.
    fn app_title(&self) -> &str {
        "VaultDrive v3.0"
    }

    /// Status pill rendered next to `app_title` in the header bar.
    fn status_badge(&self) -> (&str, Color) {
        if self.syncing {
            ("↻ SYNCING", Color::Cyan)
        } else if self.mounted {
            ("● MOUNTED", Color::Green)
        } else {
            ("○ UNMOUNTED", Color::Yellow)
        }
    }

    /// Priority-ordered telemetry segments for the top header bar.
    /// Higher `priority` (`0..=255`) survives longer when the terminal window is narrow!
    fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
        let up_last = self.up_samples.last().copied().unwrap_or(0.0);
        let down_last = self.down_samples.last().copied().unwrap_or(0.0);
        let rate_text = format!(
            "▲ {} ▼ {}",
            format_rate(up_last).as_str(),
            format_rate(down_last).as_str()
        );
        let cache_text = format!(
            "VFS: {} / {}",
            format_bytes(self.cache_used_bytes).as_str(),
            format_bytes(self.cache_max_bytes).as_str()
        );
        vec![
            PrioritySegment::new(self.remote_path.as_str(), Style::DEFAULT, 95),
            PrioritySegment::new(rate_text, Style::DEFAULT, 80),
            PrioritySegment::new(cache_text, Style::DEFAULT, 50),
        ]
    }

    /// Called automatically on every frame tick (e.g. every 250ms).
    fn on_tick(&mut self) {
        // Push latest upload/download throughput samples and update WaveScaler:
        // Feed the whole visible window of BOTH series (shared ceiling), never just the newest sample:
        let _ = self.scaler.update_sync(&self.up_samples, &self.down_samples);
    }

    /// Optional: persist theme/spacing changes when the user presses Enter in [19] or [20].
    fn save_ui_prefs(&mut self) -> Result<(), TuiError> {
        Ok(())
    }
}
```

---

### Step 2: Build Display Widgets in `src/tui/addons/` (`impl SectionWidget<B>`)

File references:
- [`src/dashboard/section.rs`](./src/dashboard/section.rs) (`SectionWidget`)
- [`src/components/dual_waveform.rs`](./src/components/dual_waveform.rs) (`DualWaveform`)
- [`src/components/meter.rs`](./src/components/meter.rs) (`GradientMeter`)
- [`src/widgets/weighted_table.rs`](./src/widgets/weighted_table.rs) (`WeightedTable`)

> **IMPORTANT FOR AI AGENTS**:
> `Section::display(Box::new(MyWidget))` **automatically** draws the outer `TabPanel` border (`╭─┐ Title ┌──╮`), title notch, subtitle notch, focus border highlight, and 4-directional `option_gap` inset!
> Your `SectionWidget::render` method receives `inner: Rect` — which is **already the interior content area inside the panel**. Do NOT draw another `TabPanel` inside `SectionWidget::render`.

#### Example 2A: Bandwidth Waveform Widget (`src/tui/addons/wave_panel.rs`)
```rust
use dashboard_tui::prelude::*;
use crate::app::VaultDriveApp;

pub struct SyncWaveformWidget;

impl SectionWidget<VaultDriveApp> for SyncWaveformWidget {
    fn title(&self, _backend: &VaultDriveApp) -> String {
        "Bandwidth Telemetry".to_string()
    }

    fn subtitle(&self, _backend: &VaultDriveApp) -> String {
        "60s Window".to_string()
    }

    fn render(
        &self,
        backend: &VaultDriveApp,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        // Zero-heap-allocation rate formatting via StackStr<32>:
        let up_now = format_rate(backend.up_samples.last().copied().unwrap_or(0.0));
        let dn_now = format_rate(backend.down_samples.last().copied().unwrap_or(0.0));
        let mode = if tty_mode {
            GraphSymbolMode::Tty
        } else {
            GraphSymbolMode::Braille
        };

        // DualWaveform automatically stretches/compresses samples across 100% of `inner.width()`
        // and adapts to any window resize (both horizontally and vertically).
        DualWaveform::new(
            &backend.up_samples,
            &backend.down_samples,
            backend.scaler.current_scale(),
        )
        .with_labels(up_now.as_str(), dn_now.as_str())
        .with_mode(mode)
        .render(buffer, inner, palette);
    }
}
```

#### Example 2B: Key-Value + Gradient Meter Status Widget (`src/tui/addons/status_panel.rs`)
```rust
use dashboard_tui::prelude::*;
use crate::app::VaultDriveApp;

pub struct DriveStatusWidget;

impl SectionWidget<VaultDriveApp> for DriveStatusWidget {
    fn title(&self, _backend: &VaultDriveApp) -> String {
        "Drive & VFS Status".to_string()
    }

    fn subtitle(&self, backend: &VaultDriveApp) -> String {
        if backend.mounted { "FUSE3 Ready".into() } else { "Offline".into() }
    }

    fn render(
        &self,
        backend: &VaultDriveApp,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        if inner.is_empty() {
            return;
        }
        let x0 = inner.x();
        let max_w = inner.width().get();
        let mut y = inner.y();

        // Draw key-value rows using `buffer.set_string(col, row, text, style, max_width)`
        let _ = buffer.set_string(x0, y, "Remote:", Style::new().with_fg(palette.muted), max_w);
        let _ = buffer.set_string(
            x0.saturating_add(12),
            y,
            backend.remote_path.as_str(),
            Style::new().with_fg(palette.accent).bold(),
            max_w.saturating_sub(12),
        );
        y = y.saturating_add(1);

        // Draw a smooth gradient progress bar `[■■■■□□□□] 32%`
        if y.get() < inner.bottom().get() && max_w > 16 {
            let ratio = (backend.cache_used_bytes as f64) / (backend.cache_max_bytes.max(1) as f64);
            let _ = buffer.set_string(x0, y, "VFS Cache:", Style::new().with_fg(palette.muted), 11);
            let bar_w = max_w.saturating_sub(13).min(24);
            let _ = GradientMeter::new(ratio, bar_w)
                .with_tty_mode(tty_mode)
                .render(buffer, x0.saturating_add(12), y, palette);
        }
    }
}
```

#### Example 2C: Proportional Table Widget (`WeightedTable`)
```rust
use dashboard_tui::prelude::*;
use crate::app::VaultDriveApp;

pub struct TransfersTableWidget;

impl SectionWidget<VaultDriveApp> for TransfersTableWidget {
    fn title(&self, _backend: &VaultDriveApp) -> String {
        "Active VFS Transfers".to_string()
    }

    fn render(
        &self,
        _backend: &VaultDriveApp,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        _tty_mode: bool,
    ) {
        let cols = [
            TableColumn::new("Object Path", 4).with_min_width(14),
            TableColumn::new("Direction", 2).with_min_width(8),
            TableColumn::new("Size", 2).with_min_width(8).with_alignment(TextAlignment::Right),
            TableColumn::new("Progress", 2).with_min_width(8).with_alignment(TextAlignment::Right),
        ];
        let rows: [&[&str]; 2] = [
            &["backups/db.tar.zst", "UPLOAD", "1.42 GB", "78%"],
            &["keys/master.age",    "VERIFY", "4.00 KB", "100%"],
        ];
        WeightedTable::new(&cols, &rows)
            .with_selected_row(Some(0))
            .render(buffer, inner, palette);
    }
}
```

---

### Step 3: Build Menu Options in `src/tui/addons/` (`OptionSpec<B>`)

File references:
- [`src/dashboard/section.rs`](./src/dashboard/section.rs) (`OptionSpec`, `OptionOutcome`)
- [`src/universal/`](./src/universal/mod.rs) (`EditModal`, `ConfirmModal`, `TypedModal`, `NoticeModal`)

Every numbered menu option (`[1]`..`[99]`) is constructed with `OptionSpec::new(number, label, handler)` and returns an [`OptionOutcome<B>`](./src/dashboard/section.rs#L94-L115):

| `OptionOutcome` Variant | What the Engine Does |
| :--- | :--- |
| `OptionOutcome::None` | Silent state mutation; redraws on next frame |
| `OptionOutcome::Toast { message, is_error }` | Shows a 4-second status/error toast in the footer bar |
| `OptionOutcome::OpenModal(Box<dyn Modal<B>>)` | Opens a centered modal overlay with dimmed background |
| `OptionOutcome::PushSubView(Box<dyn SubView<B>>)` | Pushes a full-screen drill-down view onto the `SubView` stack |
| `OptionOutcome::Quit` | Cleanly exits the dashboard session |

#### Example: All 5 Option Patterns (`src/tui/addons/quick_actions.rs`)
```rust
use dashboard_tui::prelude::*;
use crate::app::VaultDriveApp;
use crate::tui::addons::log_inspector::LiveJournalSubView;

/// Pattern 1: Immediate toggle + Footer Toast + Dynamic right-aligned status badge
pub fn toggle_mount_option(num: u16) -> OptionSpec<VaultDriveApp> {
    OptionSpec::new(num, "Mount / Unmount Vault", |ctx| {
        ctx.backend.mounted = !ctx.backend.mounted;
        OptionOutcome::Toast {
            message: if ctx.backend.mounted {
                "Vault mounted".into()
            } else {
                "Vault unmounted".into()
            },
            is_error: false,
        }
    })
    .with_badge_fn(|app: &VaultDriveApp| {
        if app.mounted { "MOUNTED".into() } else { "OFFLINE".into() }
    })
}

/// Pattern 2: Text Input Modal (`EditModal`) with validation
pub fn edit_remote_option(num: u16) -> OptionSpec<VaultDriveApp> {
    OptionSpec::new(num, "Edit Remote Path", |ctx| {
        let current = ctx.backend.remote_path.clone();
        OptionOutcome::OpenModal(Box::new(EditModal::new(
            "Edit Remote Path",
            "Specify encrypted rclone remote target:",
            current,
            |app: &mut VaultDriveApp, new_val: &str| {
                if new_val.trim().is_empty() {
                    return Err("Remote path cannot be empty".into());
                }
                app.remote_path = new_val.trim().to_string();
                Ok(format!("Remote updated to {}", app.remote_path))
            },
        )))
    })
    .with_badge_fn(|app: &VaultDriveApp| app.remote_path.clone())
}

/// Pattern 3: Yes/No Confirmation Modal (`ConfirmModal`)
pub fn purge_cache_option(num: u16) -> OptionSpec<VaultDriveApp> {
    OptionSpec::new(num, "Purge VFS Disk Cache", |_ctx| {
        OptionOutcome::OpenModal(Box::new(ConfirmModal::new(
            "Purge VFS Cache",
            "Evict all clean cached chunks from local NVMe storage?",
            |app: &mut VaultDriveApp| {
                app.cache_used_bytes = 0;
                Ok("Purged VFS cache".into())
            },
        )))
    })
}

/// Pattern 4: Destructive Phrase-Match Modal (`TypedModal`)
pub fn factory_reset_option(num: u16) -> OptionSpec<VaultDriveApp> {
    OptionSpec::new(num, "Factory Reset Config", |_ctx| {
        OptionOutcome::OpenModal(Box::new(TypedModal::new(
            "Factory Reset Configuration",
            "Type RESET below to restore default mount settings:",
            "RESET",
            |_app: &mut VaultDriveApp| Ok("Restored factory defaults".into()),
        )))
    })
}

/// Pattern 5: Push Full-Area Drill-Down View (`SubView`)
pub fn open_journal_option(num: u16) -> OptionSpec<VaultDriveApp> {
    OptionSpec::new(num, "Open Live Systemd Journal", |_ctx| {
        OptionOutcome::PushSubView(Box::new(LiveJournalSubView))
    })
}
```

---

### Step 4: Build a Drill-Down `SubView` in `src/tui/addons/` (`impl SubView<B>`)

File reference: [`src/dashboard/sub_view.rs`](./src/dashboard/sub_view.rs)

When a `SubView` is active, it replaces the page body area between the top header bar and bottom footer bar. Pressing `Esc` pops back to the dashboard page.

```rust
use dashboard_tui::prelude::*;
use crate::app::VaultDriveApp;

pub struct LiveJournalSubView;

impl SubView<VaultDriveApp> for LiveJournalSubView {
    fn title(&self) -> &str {
        "Systemd User Journal (vaultdrive.service)"
    }

    fn render(
        &self,
        _backend: &VaultDriveApp,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        // Unlike SectionWidget, a SubView owns the entire body `area`, so wrap in a `TabPanel`:
        let inner = TabPanel::new(self.title())
            .with_subtitle("Esc return")
            .with_focused(true)
            .with_tty_mode(tty_mode)
            .render(buffer, area, palette);

        let _ = buffer.set_string(
            inner.x(),
            inner.y(),
            "06:15:01 systemd[1420]: Started VaultDrive FUSE3 Encrypted Cloud Mount.",
            Style::new().with_fg(palette.text),
            inner.width().get(),
        );
    }

    fn on_key(&mut self, _backend: &mut VaultDriveApp, key: KeyPress) -> SubViewResult {
        if key.code() == KeyCode::Esc || key.code() == KeyCode::Char('q') {
            SubViewResult::Pop
        } else {
            SubViewResult::Continue
        }
    }
}
```

---

### Step 5: Assemble Everything in `src/tui/dashboard.rs` (The Minimal Blueprint)

File reference: [`src/dashboard/registry.rs`](./src/dashboard/registry.rs)

```rust
// src/tui/dashboard.rs
use std::time::Duration;
use dashboard_tui::prelude::*;
use crate::app::VaultDriveApp;
use crate::tui::addons::{quick_actions, status_panel, wave_panel};

pub fn run_dashboard(app: VaultDriveApp) -> Result<(), TuiError> {
    AddonRegistry::new()
        // ── 0. Configure Shortkeys (concise & readable, or omit for defaults) ──
        .with_keys(
            KeyBindings::default()
                .quit("ctrl+q")
                .page_cycle("Shift+Tab", "Tab")
                .section_cycle("Right", "Left")
                .up_down("Up", "Down")
                .vim_up_down('k', 'j')
                .activate("Enter")
                .cancel('q'),
        )

        // ── 1. Declare Pages (F1, F2, or any custom key hint like "m", "s") ──
        .with_page(0, "Monitor",  "F1", SectionLayoutMode::Auto)
        .with_page(1, "Settings", "F2", SectionLayoutMode::Rows)

        // ── 2. Page 0 (F1: Monitor) ─────────────────────────────────────────
        // The last `bool` argument is `selectable`:
        //   - `false`: Informative visual widget; pressing `Tab` skips over it!
        //   - `true`:  Interactive section; pressing `Tab` focuses/selects it!
        .with_display_section(0, 0, status_panel::DriveStatusWidget, 6, 8, false)
        .with_display_section(0, 1, wave_panel::SyncWaveformWidget,  6, 8, false)
        .with_menu_section(0, 2, "Quick Actions", "1-4 run", 6, 10, true)
        .with_option(0, 2, quick_actions::toggle_mount_option(1))
        .with_option(0, 2, quick_actions::purge_cache_option(2))
        .with_option(0, 2, quick_actions::open_journal_option(3))

        // ── 3. Page 1 (F2: Settings) + Tier 2 Universal Add-Ons ─────────────
        .with_menu_section(1, 0, "Preferences", "Enter edit", 10, 20, true)
        .with_option(1, 0, quick_actions::edit_remote_option(1))
        .with_option(1, 0, quick_actions::factory_reset_option(18))
        .with_option(1, 0, section_gap_option(19))
        .with_option(1, 0, color_theme_option(20))

        // ── 4. Build & Launch Interactive Session ───────────────────────────
        .build_engine(app)
        .run_interactive(Duration::from_millis(250))
}
```

---

## 5. Layout Modes, Built-In Keyboard Routing, & Visual Invariants

### 5.1 `SectionLayoutMode` Reference ([`src/dashboard/page.rs`](./src/dashboard/page.rs))
- **`SectionLayoutMode::Auto`** *(Recommended for mixed pages)*:
  - On wide terminals (`width >= 96`) with `2 Display + 1 Menu` sections: automatically arranges the 2 Display panels side-by-side on top and the Menu panel spanning full width on the bottom (`TwoTopOneBottom`).
  - On narrow terminals (`width < 96`): stacks all sections vertically (`Rows`) and automatically drops non-critical trailing `Display` sections if terminal height is extremely tight so the interactive `Menu` remains usable.
- **`SectionLayoutMode::Rows`**: Stacks all sections vertically, distributing height between `min_height` and `preferred_height`.
- **`SectionLayoutMode::Columns`**: Places all sections side-by-side in equal-width columns.
- **`SectionLayoutMode::TwoTopOneBottom`**: 2 sections side-by-side on top, 1 full-width section on the bottom (degrades to `Rows` when `width < 76`).
- **`SectionLayoutMode::OneTopTwoBottom`**: 1 full-width section on top, 2 side-by-side on the bottom.
- **`SectionLayoutMode::Grid2x2`**: 2×2 quadrant grid for 4 sections.

### 5.2 Key Mapping Summary & Shortkey Customization (`KeyBindings`)

Every shortkey in the dashboard can be configured concisely in the dashboard project file using `KeyBindings::default()`:

| Key | Action |
|:---|:---|
| `→ / ←` (Right / Left Arrows) | Move between sections on the active page |
| `Tab / Shift+Tab` | Move between pages (F1 ↔ F2 ↔ F3) |
| `q` | Cancel: closes a modal, pops a sub-view; at the dashboard root `handle_key` returns `EngineAction::Back` (leave the UI, keep services running) |
| `^Q` (`Ctrl+Q`) | `EngineAction::Quit`: terminate the whole app (host stops its services) |
| `Ctrl+C` | `EngineAction::Back` |
| `↑ / ↓` (or `k / j`) | Move cursor between options inside the active section |
| `Enter` | Activate selected option |

`run_interactive` returns the `EngineAction` that ended the loop (`Back` or `Quit`), so the host decides what each means. Text-input modals return `ModalResult::Consumed` for typed characters, so the cancel key (`q`) can be typed into a field instead of closing it; custom text modals should do the same. `DashboardEngine::has_sub_view()` and `focused_section()` expose read-only UI state for hosts and tests.

Developers can enter any `ctrl+[key]` of their liking in the dashboard project file using intuitive strings (e.g. `"ctrl+q"`, `"^q"`, `"ctrl-x"`, `"ctrl+w"`), characters, or `KeyPress`:

```rust
KeyBindings::default()
    .quit("ctrl+q")                  // Any ctrl+[key] e.g. "^q", "ctrl-x", "ctrl+w"
    .page_cycle("Shift+Tab", "Tab")  // Backward / Forward page cycling
    .section_cycle("Right", "Left")  // Next / Previous selectable section
    .up_down("Up", "Down")           // Menu cursor movement
    .vim_up_down('k', 'j')           // Vim-style secondary cursor movement
    .activate("Enter")               // Option activation
    .cancel('q')                     // Cancel modal / pop subview / clear toast
```

#### Warning popups on a key & extra option shortkeys

```rust
// Warn before quitting: Enter confirms, q / Esc cancels (Ctrl+C always leaves at once (`Back`)).
KeyBindings::default().quit("ctrl+q").confirm_quit(true)

// The developer decides the popup text (default: "Quit" / "Are you sure you want to quit?"):
KeyBindings::default()
    .quit("ctrl+q")
    .confirm_quit(true)
    .quit_popup("Stop server", "This stops all running services.")

// Put a warning popup on ANY key. Enter runs the closure, q / Esc cancels.
registry.bind_confirmed_key("ctrl+x", "Wipe Cache", "This cannot be undone.", |app: &mut App| {
    app.wipe_cache();
    Ok("Cache wiped".to_string())      // Ok(msg) -> success toast, Err(msg) -> error toast
});

// Full control (any OptionOutcome: Toast, OpenModal, PushSubView, Quit):
registry.bind_key("ctrl+r", |_ctx| OptionOutcome::Quit);

// Extra shortkey for a menu option; it KEEPS its number (`7` and `o` both run it):
OptionSpec::new(7, "Open Storage", |_ctx| OptionOutcome::None).with_shortkey('o')
```

`ConfirmModal::warning(..)` (Enter pre-selects Yes) and `ConfirmModal::quit(..)` are available for custom flows via `OptionOutcome::OpenModal`. Bound keys and option shortkeys apply only on the dashboard root (no modal or sub-view open) and take priority over page/section keys, so avoid reusing `Tab`, arrows, or `q`.

#### Tab badges, footer flashes, and the log viewer

```rust
// Live badge beside the ACTIVE page's tab, e.g. " [F2:Logs 3 errors] " (empty string hides it).
Page::new("Logs", "F2").with_tab_badge(|app: &App| match app.error_count() {
    0 => String::new(),
    n => format!("{n} errors"),
})
registry.with_tab_badge(1, |app| ...)          // same, for a registry page index

// Scrollable newest-first log page (read-only view over YOUR Vec<LogEntry>; no copying):
OptionOutcome::PushSubView(Box::new(LogViewSubView::new("Live Logs", |app: &App| app.log.as_slice())))

// Footer feedback: `show_toast` = result (stays 6s), `show_hint` = nudge (stays 2.5s).
// Pressing an unmapped key shows "No shortcut for 'x'. Use …"; `?` shows a one-line help.
engine.with_invalid_key_hint(|key| Some(format!("{} does nothing", key.short_label().as_str())))
engine.with_invalid_key_hints(false)           // silence the default hint
```

#### Ready-made telemetry panels (`StatusPanel`, `ThroughputPanel`)

Zero-boilerplate `SectionWidget`s driven by plain `fn(&App) -> …` accessors (nothing is copied or allocated per frame):

```rust
// Labelled rows: gradient meters (+ optional detail), plain text, and ▲/▼ rates.
let system = StatusPanel::<App>::new("CloudVault System")
    .with_subtitle("cloudvault.service")
    .with_badge(|app| if app.online { ("● ONLINE", Some(Color::Green)) } else { ("● OFFLINE", Some(Color::Red)) })
    .with_row(StatusRow::new("CPU", |app| StatusValue::Meter(app.cpu_pct)))
    .with_row(StatusRow::new("RAM", |app| StatusValue::Meter(app.mem_pct))
        .with_detail(|app, out| { let _ = write!(out, "({:.1}G)", app.mem_total_gb); }))
    .with_row(StatusRow::new("Daemon", |app| StatusValue::Text(app.daemon_state.as_str())))
    .with_row(StatusRow::new("Net", |app| StatusValue::Rates { up_bytes_per_sec: app.tx, down_bytes_per_sec: app.rx }));

// Live waveform with a rate header and an automatic ● ACTIVE / ● IDLE badge.
let disk = ThroughputPanel::<App>::dual("Disk I/O", "Write", |a| a.write_hist.as_slice(), "Read", |a| a.read_hist.as_slice());
let net  = ThroughputPanel::<App>::single("Network Monitor", "Traffic", |a| a.traffic_hist.as_slice());
page.push_section(Section::display(Box::new(system)).with_heights(8, 10));
```

The host appends one sample per `DashboardBackend::on_tick` to its own `Vec<f64>`; the panels read the newest samples and pick a "nice" scale from the visible peak.

#### Settings forms, drill-down pages, and helper types

```rust
// Multi-field form: edits preview live, Esc/q restores every value, Enter runs the save hook.
OptionOutcome::OpenModal(Box::new(
    SettingsModal::<App>::new("Server Settings")
        .with_number("Workers", 1..=32, |a| a.workers, |a, v| a.workers = v)
        .with_toggle("TLS", |a| a.tls, |a, on| a.tls = on)
        .with_choice("Log level", ["Debug", "Info", "Warn"], |a| a.level, |a, i| a.level = i)
        .on_save(|a| a.write_config().map(|()| "Saved".into()).map_err(|e| e.to_string())),
))

// Drill-down page made of ordinary sections/options (options may open modals or deeper pages):
let mut net = Section::menu("Network", "1 run", Vec::new()).with_heights(5, 8);
net.push_option(OptionSpec::new(1, "Show address", |_| OptionOutcome::Toast { message: "0.0.0.0:80".into(), is_error: false }));
OptionOutcome::PushSubView(Box::new(PageSubView::new("Network").with_section(net)))

// Helpers so the host keeps no ring-buffer code:
RateHistory::new(120)   // .push(sample) each tick; .as_slice() feeds ThroughputPanel; .latest()
LogBuffer::new(500)     // .info/.warn/.error(cat, msg); .as_slice() feeds LogViewSubView; .count_level(..)
```

- `Space` is a second default activate key (`KeyBindings::alt_activate(..)` / `without_alt_activate()`).
- Custom `Modal`s can override `render_with_backend` to draw live backend values, and `SubView`s can override `on_key_with_context` to open modals or push deeper sub-views via `SubViewAction::Outcome(OptionOutcome)`. `ModalContext::keys` exposes the active `KeyBindings`.
- A complete second dashboard for an unrelated app lives in `examples/webserver_dashboard.rs` (~120 lines of declarations); copy it as a starting point.

#### Terminal background (on / off / hardcoded)

```rust
// Off (default): the terminal application's own background (and transparency) is untouched.
// Theme: every cell without its own background gets the active theme's background.
// Fixed: one hardcoded color, whatever the theme.
registry.with_terminal_background(TerminalBackground::Theme)
engine.set_terminal_background(TerminalBackground::Fixed(Color::Rgb(0x10, 0x10, 0x18)))   // at runtime

// Let the END USER flip it in the app with a ready-made live-preview option:
registry.with_option(1, 0, terminal_background_option(21))
```

Every built-in palette carries a `background`; custom code can read `palette.background`.

#### General-purpose widgets (`Widget` / `StatefulWidget`)

Every standard widget draws into any `Rect` of a `Buffer` through two small traits, so they work inside sections, modals, sub-views, or a hand-written loop. They carry their own `Style`s (nothing is themed or hardcoded); pass colors from `ColorPalette` or write them with `Stylize`.

```rust
use dashboard_tui::prelude::*;

// Layout: Ratio / Flex / areas::<N>() (destructure, no indexing, never panics)
let [header, body, footer] = Layout::vertical([Constraint::Fixed(1), Constraint::Fill(1), Constraint::Fixed(1)]).areas(area);
let [sidebar, main] = Layout::horizontal([Constraint::Ratio(1, 4), Constraint::Fill(1)]).with_gap(1).areas(body);
let buttons = Layout::horizontal([Constraint::Fixed(8); 3]).with_flex(Flex::SpaceBetween).split(footer);

// Fluent styles
let warning = "disk almost full".yellow().bold();

// Stateful list / table: the app owns the state, widgets scroll to keep the selection visible
let mut list_state = ListState::default().with_selected(Some(0));      // select_next(), select_previous(), ...
buffer.render_stateful_widget(List::new(["mount", "sync"]).highlight_symbol("> "), sidebar, &mut list_state);
let mut table_state = TableState::default();
buffer.render_stateful_widget(
    Table::new([Row::new(["db.tar", "78%"])], [Constraint::Fill(1), Constraint::Fixed(5)])
        .header(Row::new(["File", "Done"])).highlight_symbol("> "),
    main, &mut table_state);

// Plain widgets
buffer.render_widget(Paragraph::new("long text...").wrap(WrapMode::Word), main);     // .line_count(width) for scrollbars
buffer.render_widget(Tabs::new(["Monitor", "Setup"]).select(0), header);
buffer.render_widget(Gauge::default().ratio(0.4).gauge_style(Style::new().bg(Color::Green)), footer);
buffer.render_widget(Sparkline::new(history.as_slice()), main);                      // newest samples, right-aligned
buffer.render_widget(BarChart::new([Bar::new("cpu", 62)]).show_values(true), main);
buffer.render_widget(Chart::new([Dataset::new("load", &points).graph_type(GraphType::Line)])
    .x_axis(Axis::new([0.0, 60.0]).labels(["0s", "60s"])).y_axis(Axis::new([0.0, 100.0]).labels(["0", "100"])), main);
buffer.render_widget(Canvas::new([0.0, 10.0], [0.0, 10.0]).shape(Shape::Circle { x: 5.0, y: 5.0, radius: 3.0, style: Style::DEFAULT }), main);
buffer.render_widget(Calendar::new(2026, 11).selected(Some(8)), main);
buffer.render_widget(Clear, popup_area);                                              // wipe before drawing a popup
```

See `examples/widget_gallery.rs` for all of them on one screen (`cargo run --example widget_gallery`). All widgets are property-tested to never panic for any geometry, NaN/infinite data, or off-buffer areas.

#### Cursor, inline viewport, and box-drawing merges

```rust
// Show the hardware cursor (e.g. in a text field); None hides it again.
session.draw_with_cursor(|buffer, area| { /* draw */ Some((ColumnIndex::new(10), RowIndex::new(3))) })?;

// Draw in the last N rows of the normal screen and leave earlier terminal output untouched.
let config = TerminalSessionConfig::new().with_viewport(Viewport::Inline(12));
engine.run_interactive_with(config, Duration::from_millis(250))?;

// Join adjacent borders cleanly: "─" over "│" becomes "┼" (light/rounded, heavy, double).
buffer.set_border_symbol(column, row, "│", Style::DEFAULT);
```

- **`selectable` (`true` / `false`) Section Focus**: Cycle focus (`→ / ←`) **only** among sections on the current page that have `selectable = true` (configured via `.with_display_section(..., selectable)`, `.with_menu_section(..., selectable)`, or `Section::with_selectable(bool)`). Purely informative visual sections with `selectable = false` are never selected.
- **`1`..`99` Numeric Direct Jump**:
  - If the page's highest option number is `<= 9`: pressing `1`..`9` **immediately** executes that option.
  - If the page's highest option number is `>= 10` (e.g. `1`..`20`): pressing `1` or `2` opens a `450ms` prefix window (`#1_` or `#2_` shown in the footer bar). Pressing a second digit (`0`..`9`) immediately dispatches the 2-digit option (`10`..`20`), or pressing `Enter` / waiting `450ms` dispatches the single-digit option (`1` or `2`).
- **`Enter`**: Executes the currently highlighted menu option.
- **`^Q` / `Ctrl+C`**: Cleanly exits the dashboard and restores terminal state.

---

## 6. Headless Verification Recipe for AI Agents

**NEVER** guess whether a dashboard layout renders properly. Every dashboard built with `dashboard-tui` can and MUST be verified headlessly in unit/integration tests using [`TestBackend`](./src/backend/test_backend.rs) and [`Buffer::to_visual_lines()`](./src/core/buffer.rs#L406-L423):

```rust
#[test]
fn verify_project_dashboard_headless() {
    use dashboard_tui::prelude::*;

    let mut engine = build_my_project_engine(); // Your AddonRegistry::new()...build_engine(app)

    // Test across compact (80x24), standard (120x32), and ultra-wide (180x44) viewports:
    for (w, h) in [(80, 24), (120, 32), (180, 44)] {
        let area = Rect::from_size(TerminalWidth::new(w), TerminalHeight::new(h));
        let mut buf = Buffer::empty(area);

        engine.tick();
        engine.render(&mut buf, area);

        let lines = buf.to_visual_lines();
        assert_eq!(lines.len(), usize::from(h));
        assert!(lines[0].contains("VaultDrive"));

        // Simulate key presses deterministically:
        assert_eq!(engine.handle_key(KeyPress::new(KeyCode::F(2))), EngineAction::Continue);
        engine.render(&mut buf, area);
    }
}
```

### Verification Commands Checklist
When working inside `/home/cee/Documents/dashboard-tui` or a project consuming `dashboard-tui`, run:
```bash
cargo clippy --all-targets --all-features
cargo test
cargo doc --no-deps
cargo run --example vaultdrive_dashboard -- --headless-check
```

---

## 7. QR Code Generation & Clickable Terminal Web-Links (OSC 8)

### 7.1 Generating & Rendering QR Codes
`dashboard-tui` supports zero-dependency QR code generation and 2× half-block rendering directly within the library:

```rust
use dashboard_tui::prelude::*;

// 1. Generate an owned QR matrix from text or a URL:
let qr_matrix = QrCodeMatrix::from_text("https://github.com/CeeSmag/dashboard-tui")?;

// 2. Adjust quiet zone or custom colors (optional):
let qr_matrix = qr_matrix
    .with_quiet_zone(2)
    .with_colors(Color::Black, Color::White);

// 3. Render directly into any Buffer:
qr_matrix.render(&mut buffer, ColumnIndex::new(5), RowIndex::new(2));

// 4. Or embed directly inside a dashboard page as a SectionWidget:
let qr_section = Section::display(Box::new(
    QrWidget::from_text("Mobile Companion Pairing", "https://vaultdrive.local/pair")?
        .with_subtitle("Scan with mobile camera")
))
.with_heights(8, 14)
.with_selectable(false);
```

### 7.2 Clickable Terminal Hyperlinks (OSC 8)
Modern terminal emulators (iTerm2, Alacritty, Kitty, `WezTerm`, VS Code, Windows Terminal) support clickable web links using standard OSC 8 escape sequences. In `dashboard-tui`, hyperlinks are first-class primitives across `Cell`, `Span`, `Buffer`, and `AnsiFrameWriter`:

```rust
use dashboard_tui::prelude::*;

// 1. In inline text runs (Span / Line):
let link_span = Span::hyperlink("Open Documentation", "https://github.com/CeeSmag/dashboard-tui");
let line = Line::from_spans([Span::raw("For help: "), link_span]);
buffer.set_line(ColumnIndex::new(0), RowIndex::new(0), &line, TerminalWidth::new(40));

// 2. Using the Hyperlink widget:
let link_widget = Hyperlink::new("https://crates.io/crates/dashboard-tui", "View on Crates.io");
link_widget.render(&mut buffer, ColumnIndex::new(0), RowIndex::new(1), 30);

// 3. Directly in cell buffers:
buffer.set_hyperlink(
    ColumnIndex::new(0),
    RowIndex::new(2),
    "https://rust-lang.org",
    "Rust Website",
    Style::new().fg(Color::Cyan).underlined(),
    30,
);

// 4. Standalone string formatters for toasts, modals, and notifications:
let toast = format_hyperlink("https://github.com/CeeSmag/dashboard-tui", "Click to visit repo");
```
