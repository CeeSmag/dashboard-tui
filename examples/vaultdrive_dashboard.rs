//! Complete 3-Page `VaultDrive` Dashboard Showcase (`examples/vaultdrive_dashboard.rs`).
//!
//! Demonstrates how the full Rust rewrite of `VaultDrive` (`/home/cee/Documents/vaultdrive-linux`)
//! plugs into `tui-rs`:
//! - **Page 1 (`F1: Monitor`)**:
//!   - `DriveStatusWidget` (mount status, VFS cache meter bar, remote quota, systemd unit status).
//!   - `SyncWaveformWidget` (`DualWaveform` with `WaveScaler` upload/download history).
//!   - `QuickActions` menu (`[1]` Mount/Unmount, `[2]` Sync Now, `[3]` Verify Integrity,
//!     `[4]` Clear VFS Cache, `[5]` View Live Journal).
//! - **Page 2 (`F2: Settings & Security`)**:
//!   - 20 numbered options (`[1]`..`[20]`) demonstrating 2-column auto-layout (`width >= 100`),
//!     `EditModal`, `ConfirmModal`, `TypedModal`, `NoticeModal`, plus the Tier 2 Universal
//!     Add-Ons `[19] Section Spacing Gaps` and `[20] Color Theme`.
//! - **Page 3 (`F3: Diagnostics & Logs`)**:
//!   - `ActiveTransfersWidget` (`WeightedTable` of in-flight rclone transfers).
//!   - `DiagnosticsMenu` with drill-down `SubView` log inspector.
//!
//! Run interactively in a terminal:
//! ```bash
//! cargo run --example vaultdrive_dashboard
//! ```
//! Or run a deterministic headless verification pass:
//! ```bash
//! cargo run --example vaultdrive_dashboard -- --headless-check
//! ```

use std::time::{Duration, Instant};

use tui_rs::backend::{Backend, CrosstermBackend, TestBackend, format_hyperlink};
use tui_rs::components::{
    DualWaveform, GradientMeter, GraphSymbolMode, QrWidget, WaveScaler, WaveScalerConfig,
    format_bytes, format_rate,
};
use tui_rs::core::{
    Buffer, Color, ColorPalette, PrioritySegment, Rect, Style, TerminalHeight, TerminalWidth,
    TextAlignment,
};
use tui_rs::dashboard::{
    AddonOptionEntry, AddonPageEntry, AddonRegistry, AddonSectionEntry, DashboardBackend,
    DashboardEngine, EngineAction, KeyBindings, KeyCode, KeyPress, ModalContext, OptionOutcome,
    OptionSpec, Section, SectionLayoutMode, SectionWidget, SubView, SubViewResult,
};
use tui_rs::terminal::{
    TerminalSessionConfig, TerminalSessionGuard, install_panic_restoration_hook,
};
use tui_rs::universal::{
    ConfirmModal, EditModal, NoticeModal, TypedModal, color_theme_option, section_gap_option,
};
use tui_rs::widgets::{TabPanel, TableColumn, WeightedTable};

struct VaultDriveMockBackend {
    mounted: bool,
    syncing: bool,
    remote_path: String,
    mount_point: String,
    vfs_cache_mode: String,
    bandwidth_limit: String,
    cache_used_bytes: u64,
    cache_max_bytes: u64,
    up_samples: Vec<f64>,
    down_samples: Vec<f64>,
    scaler: WaveScaler,
    tick_counter: u64,
}

/// Number of 250ms samples in the 60-second telemetry window (`240` samples).
const TELEMETRY_WINDOW_SAMPLES: usize = 240;

#[expect(clippy::cast_precision_loss, reason = "sample index is small")]
fn sample_pair_at_tick(tick: u64) -> (f64, f64) {
    let t = (tick as f64) * 0.18;
    let up_a = t.sin().mul_add(0.5, 0.5);
    let up_b = (t * 2.3).cos().mul_add(0.5, 0.5);
    let up_wave = up_a.mul_add(920_000.0, up_b.mul_add(480_000.0, 120_000.0));

    let dn_a = t.mul_add(0.85, 1.2).sin().mul_add(0.5, 0.5);
    let dn_b = (t * 1.9).sin().mul_add(0.5, 0.5);
    let down_wave = dn_a.mul_add(1_050_000.0, dn_b.mul_add(520_000.0, 150_000.0));
    (up_wave, down_wave)
}

impl Default for VaultDriveMockBackend {
    fn default() -> Self {
        let mut up = Vec::with_capacity(TELEMETRY_WINDOW_SAMPLES);
        let mut down = Vec::with_capacity(TELEMETRY_WINDOW_SAMPLES);
        for tick in 0..(TELEMETRY_WINDOW_SAMPLES as u64) {
            let (u, d) = sample_pair_at_tick(tick);
            up.push(u);
            down.push(d);
        }
        let mut scaler = WaveScaler::new(WaveScalerConfig::new(65_536.0));
        let _ = scaler.update(&down);
        Self {
            mounted: true,
            syncing: false,
            remote_path: "gdrive-crypt:/Vault".to_string(),
            mount_point: "/home/user/VaultDrive".to_string(),
            vfs_cache_mode: "writes".to_string(),
            bandwidth_limit: "Unlimited".to_string(),
            cache_used_bytes: 3_489_660_928,
            cache_max_bytes: 10_737_418_240,
            up_samples: up,
            down_samples: down,
            scaler,
            tick_counter: TELEMETRY_WINDOW_SAMPLES as u64,
        }
    }
}

impl DashboardBackend for VaultDriveMockBackend {
    fn app_title(&self) -> &'static str {
        "VaultDrive v3.0"
    }

    fn status_badge(&self) -> (&'static str, Color) {
        if self.syncing {
            ("↻ SYNCING", Color::Cyan)
        } else if self.mounted {
            ("● MOUNTED", Color::Green)
        } else {
            ("○ UNMOUNTED", Color::Yellow)
        }
    }

    fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
        let up_last = self.up_samples.last().copied().unwrap_or(0.0);
        let down_last = self.down_samples.last().copied().unwrap_or(0.0);
        let rate_seg = format!(
            "▲ {} ▼ {}",
            format_rate(up_last).as_str(),
            format_rate(down_last).as_str()
        );
        let cache_seg = format!(
            "VFS: {} / {}",
            format_bytes(self.cache_used_bytes).as_str(),
            format_bytes(self.cache_max_bytes).as_str()
        );
        vec![
            PrioritySegment::new(self.remote_path.as_str(), Style::DEFAULT, 95),
            PrioritySegment::new(rate_seg, Style::DEFAULT, 80),
            PrioritySegment::new(cache_seg, Style::DEFAULT, 50),
        ]
    }

    fn on_tick(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        let (next_up, next_down) = if self.mounted {
            sample_pair_at_tick(self.tick_counter)
        } else {
            (0.0, 0.0)
        };
        if self.up_samples.len() >= TELEMETRY_WINDOW_SAMPLES {
            self.up_samples.remove(0);
        }
        if self.down_samples.len() >= TELEMETRY_WINDOW_SAMPLES {
            self.down_samples.remove(0);
        }
        self.up_samples.push(next_up);
        self.down_samples.push(next_down);
        let peak = next_up.max(next_down);
        let _ = self.scaler.update(&[peak]);
    }
}

struct DriveStatusWidget;

impl SectionWidget<VaultDriveMockBackend> for DriveStatusWidget {
    fn title(&self, _backend: &VaultDriveMockBackend) -> String {
        "Drive & VFS Status".to_string()
    }

    fn subtitle(&self, backend: &VaultDriveMockBackend) -> String {
        if backend.mounted {
            "FUSE3 Ready".to_string()
        } else {
            "Offline".to_string()
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "byte ratio display")]
    fn render(
        &self,
        backend: &VaultDriveMockBackend,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        if inner.is_empty() {
            return;
        }
        let max_w = inner.width().get();
        let x0 = inner.x();
        let mut y = inner.y();

        let rows = [
            ("Remote:", backend.remote_path.as_str(), palette.accent),
            ("Mountpoint:", backend.mount_point.as_str(), palette.text),
            ("VFS Mode:", backend.vfs_cache_mode.as_str(), palette.ok),
            ("Bandwidth:", backend.bandwidth_limit.as_str(), palette.info),
        ];

        for (label, val, color) in rows {
            if y.get() >= inner.bottom().get() {
                break;
            }
            let cx = buffer.set_string(x0, y, label, Style::new().with_fg(palette.muted), max_w);
            let val_x = x0.saturating_add(13).max(cx.saturating_add(1));
            if val_x.get() < inner.right().get() {
                let _ = buffer.set_string(
                    val_x,
                    y,
                    val,
                    Style::new().with_fg(color).bold(),
                    inner.right().get().saturating_sub(val_x.get()),
                );
            }
            y = y.saturating_add(1);
        }

        if y.get() < inner.bottom().get() && max_w > 18 {
            let ratio = (backend.cache_used_bytes as f64) / (backend.cache_max_bytes.max(1) as f64);
            let _ = buffer.set_string(x0, y, "VFS Cache:", Style::new().with_fg(palette.muted), 12);
            let bar_x = x0.saturating_add(13);
            let bar_w = max_w.saturating_sub(14).min(24);
            let _ = GradientMeter::new(ratio, bar_w)
                .with_tty_mode(tty_mode)
                .render(buffer, bar_x, y, palette);
        }
    }
}

struct SyncWaveformWidget;

impl SectionWidget<VaultDriveMockBackend> for SyncWaveformWidget {
    fn title(&self, _backend: &VaultDriveMockBackend) -> String {
        "Bandwidth Telemetry".to_string()
    }

    fn subtitle(&self, _backend: &VaultDriveMockBackend) -> String {
        "60s Window".to_string()
    }

    fn render(
        &self,
        backend: &VaultDriveMockBackend,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        tty_mode: bool,
    ) {
        let up_now = format_rate(backend.up_samples.last().copied().unwrap_or(0.0));
        let dn_now = format_rate(backend.down_samples.last().copied().unwrap_or(0.0));
        let mode = if tty_mode {
            GraphSymbolMode::Tty
        } else {
            GraphSymbolMode::Braille
        };
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

struct ActiveTransfersWidget;

impl SectionWidget<VaultDriveMockBackend> for ActiveTransfersWidget {
    fn title(&self, _backend: &VaultDriveMockBackend) -> String {
        "Active VFS Transfers".to_string()
    }

    fn subtitle(&self, _backend: &VaultDriveMockBackend) -> String {
        "3 streams".to_string()
    }

    fn render(
        &self,
        _backend: &VaultDriveMockBackend,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        _focused: bool,
        _tty_mode: bool,
    ) {
        let cols = [
            TableColumn::new("Object Path", 4).with_min_width(14),
            TableColumn::new("Direction", 2).with_min_width(8),
            TableColumn::new("Size", 2)
                .with_min_width(8)
                .with_alignment(TextAlignment::Right),
            TableColumn::new("Progress", 2)
                .with_min_width(8)
                .with_alignment(TextAlignment::Right),
        ];
        let rows: [&[&str]; 3] = [
            &["backups/db-2026-11-07.tar.zst", "UPLOAD", "1.42 GB", "78%"],
            &["media/architecture-spec.pdf", "DOWNLOAD", "18.4 MB", "100%"],
            &["keys/vault-master.age", "VERIFY", "4.00 KB", "100%"],
        ];
        WeightedTable::new(&cols, &rows)
            .with_selected_row(Some(0))
            .render(buffer, inner, palette);
    }
}

struct LiveJournalSubView;

impl SubView<VaultDriveMockBackend> for LiveJournalSubView {
    fn title(&self) -> &'static str {
        "Systemd User Journal (vaultdrive-mount.service)"
    }

    fn render(
        &self,
        _backend: &VaultDriveMockBackend,
        buffer: &mut Buffer,
        area: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        let inner = TabPanel::new(self.title())
            .with_subtitle("Esc return")
            .with_focused(true)
            .with_tty_mode(tty_mode)
            .render(buffer, area, palette);
        let logs = [
            "06:15:01 systemd[1420]: Started VaultDrive FUSE3 Encrypted Cloud Mount.",
            "06:15:02 rclone[1488]: VFS cache: cleaned 0 objects (3.25 GiB in use).",
            "06:15:09 rclone[1488]: backups/db-2026-11-07.tar.zst: multipart upload chunk 12/16",
            "06:15:14 vaultdrive-watchdog[1492]: Health probe OK (latency 18 ms)",
        ];
        for (i, line) in logs.iter().enumerate() {
            #[expect(clippy::cast_possible_truncation, reason = "4 log lines")]
            let row = inner.y().saturating_add(i as u16);
            if row.get() >= inner.bottom().get() {
                break;
            }
            let _ = buffer.set_string(
                inner.x(),
                row,
                line,
                Style::new().with_fg(palette.text),
                inner.width().get(),
            );
        }
    }

    fn on_key(&mut self, _backend: &mut VaultDriveMockBackend, key: KeyPress) -> SubViewResult {
        if key.code() == KeyCode::Esc || key.code() == KeyCode::Char('q') {
            SubViewResult::Pop
        } else {
            SubViewResult::Continue
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "builds the complete 3-page showcase layout with 20 settings options"
)]
fn build_vaultdrive_engine() -> DashboardEngine<VaultDriveMockBackend> {
    let mut registry = AddonRegistry::<VaultDriveMockBackend>::new().with_keys(
        KeyBindings::default()
            .quit('q')
            .page_cycle('[', ']')
            .section_cycle("Tab", "BackTab")
            .up_down("Up", "Down")
            .vim_up_down('k', 'j')
            .activate("Enter")
            .cancel("Esc"),
    );

    // Register 3 Pages (F1, F2, F3)
    registry.register_page(AddonPageEntry::new(
        0,
        "Monitor",
        "F1",
        SectionLayoutMode::Auto,
    ));
    registry.register_page(AddonPageEntry::new(
        1,
        "Settings",
        "F2",
        SectionLayoutMode::Rows,
    ));
    registry.register_page(AddonPageEntry::new(
        2,
        "Diagnostics",
        "F3",
        SectionLayoutMode::Rows,
    ));

    // Page 0: 2 Informative Display Sections (selectable = false) + 1 Interactive Menu Section (selectable = true)
    registry.register_section(AddonSectionEntry::new(
        0,
        0,
        Section::display(Box::new(DriveStatusWidget))
            .with_heights(6, 8)
            .with_selectable(false),
    ));
    registry.register_section(AddonSectionEntry::new(
        0,
        1,
        Section::display(Box::new(SyncWaveformWidget))
            .with_heights(6, 8)
            .with_selectable(false),
    ));
    registry.register_section(AddonSectionEntry::new(
        0,
        2,
        Section::menu("Quick Actions", "1-5 run", Vec::new())
            .with_heights(6, 10)
            .with_selectable(true),
    ));

    registry.register_option(AddonOptionEntry::new(
        0,
        2,
        OptionSpec::new(
            1,
            "Mount / Unmount Vault",
            |ctx: &mut ModalContext<'_, VaultDriveMockBackend>| {
                ctx.backend.mounted = !ctx.backend.mounted;
                OptionOutcome::Toast {
                    message: if ctx.backend.mounted {
                        "Mounted /home/user/VaultDrive".to_string()
                    } else {
                        "Unmounted /home/user/VaultDrive".to_string()
                    },
                    is_error: false,
                }
            },
        ),
    ));
    registry.register_option(AddonOptionEntry::new(
        0,
        2,
        OptionSpec::new(
            2,
            "Trigger Bisync Now",
            |ctx: &mut ModalContext<'_, VaultDriveMockBackend>| {
                ctx.backend.syncing = !ctx.backend.syncing;
                OptionOutcome::Toast {
                    message: if ctx.backend.syncing {
                        "Started background rclone bisync job".to_string()
                    } else {
                        "Stopped background bisync job".to_string()
                    },
                    is_error: false,
                }
            },
        ),
    ));
    registry.register_option(AddonOptionEntry::new(
        0,
        2,
        OptionSpec::new(3, "Verify Crypt Integrity", |_ctx| {
            OptionOutcome::OpenModal(Box::new(NoticeModal::new(
                "Crypt Integrity Check",
                vec![
                    "✓ Remote cipher: XChaCha20-Poly1305 + filename encryption".to_string(),
                    "✓ 1,428 objects verified against SHA-256 manifest".to_string(),
                    "✓ 0 corrupted blocks detected".to_string(),
                ],
            )))
        }),
    ));
    registry.register_option(AddonOptionEntry::new(
        0,
        2,
        OptionSpec::new(4, "Purge VFS Disk Cache", |_ctx| {
            OptionOutcome::OpenModal(Box::new(ConfirmModal::new(
                "Purge VFS Cache",
                "Evict all clean cached chunks from local NVMe storage?",
                |b: &mut VaultDriveMockBackend| {
                    b.cache_used_bytes = 64 * 1024 * 1024;
                    Ok("Purged VFS cache down to 64.0 MB".to_string())
                },
            )))
        }),
    ));
    registry.register_option(AddonOptionEntry::new(
        0,
        2,
        OptionSpec::new(5, "Open Live Systemd Journal", |_ctx| {
            OptionOutcome::PushSubView(Box::new(LiveJournalSubView))
        }),
    ));

    // Page 1: Settings & Security (Options [1]..[20], including Tier 2 Universal [19] & [20])
    registry.register_section(AddonSectionEntry::new(
        1,
        0,
        Section::menu("Configuration & Security (1-20)", "Enter edit", Vec::new())
            .with_heights(10, 22),
    ));

    registry.register_option(AddonOptionEntry::new(
        1,
        0,
        OptionSpec::new(
            1,
            "Edit Remote Path",
            |ctx: &mut ModalContext<'_, VaultDriveMockBackend>| {
                let cur = ctx.backend.remote_path.clone();
                OptionOutcome::OpenModal(Box::new(EditModal::new(
                    "Edit Remote Path",
                    "Specify encrypted rclone remote target:",
                    cur,
                    |backend: &mut VaultDriveMockBackend, val: &str| {
                        if val.trim().is_empty() {
                            return Err("Remote path cannot be empty".to_string());
                        }
                        backend.remote_path = val.trim().to_string();
                        Ok(format!("Remote updated to {}", backend.remote_path))
                    },
                )))
            },
        ),
    ));
    registry.register_option(AddonOptionEntry::new(
        1,
        0,
        OptionSpec::new(
            2,
            "Edit Local Mountpoint",
            |ctx: &mut ModalContext<'_, VaultDriveMockBackend>| {
                let cur = ctx.backend.mount_point.clone();
                OptionOutcome::OpenModal(Box::new(EditModal::new(
                    "Edit Mountpoint",
                    "Specify local FUSE3 mount directory:",
                    cur,
                    |backend: &mut VaultDriveMockBackend, val: &str| {
                        backend.mount_point = val.trim().to_string();
                        Ok(format!("Mountpoint set to {}", backend.mount_point))
                    },
                )))
            },
        ),
    ));
    registry.register_option(AddonOptionEntry::new(
        1,
        0,
        OptionSpec::new(
            3,
            "Bandwidth Cap",
            |ctx: &mut ModalContext<'_, VaultDriveMockBackend>| {
                let cur = ctx.backend.bandwidth_limit.clone();
                OptionOutcome::OpenModal(Box::new(EditModal::new(
                    "Bandwidth Limit",
                    "Enter rclone bandwidth limit (e.g. 50M or Unlimited):",
                    cur,
                    |backend: &mut VaultDriveMockBackend, val: &str| {
                        backend.bandwidth_limit = val.trim().to_string();
                        Ok(format!(
                            "Bandwidth limit set to {}",
                            backend.bandwidth_limit
                        ))
                    },
                )))
            },
        ),
    ));

    for num in 4..=17 {
        let label = match num {
            4 => "VFS Cache Mode",
            5 => "Dir Cache Time",
            6 => "Poll Interval",
            7 => "Buffer Size",
            8 => "Transfers Parallelism",
            9 => "Checkers Parallelism",
            10 => "Systemd Auto-Start",
            11 => "Watchdog Health Timer",
            12 => "Desktop Notifications",
            13 => "Conflict Resolution",
            14 => "Trash Retention Policy",
            15 => "Hardware Keyring Lock",
            16 => "SSH Agent Forwarding",
            _ => "Audit Log Level",
        };
        registry.register_option(AddonOptionEntry::new(
            1,
            0,
            OptionSpec::new(num, label, move |_ctx| OptionOutcome::Toast {
                message: format!("Updated option [{num}] ({label})"),
                is_error: false,
            }),
        ));
    }

    registry.register_option(AddonOptionEntry::new(
        1,
        0,
        OptionSpec::new(18, "Factory Reset Vault Config", |_ctx| {
            OptionOutcome::OpenModal(Box::new(TypedModal::new(
                "Factory Reset Configuration",
                "This will reset all local mount preferences (cloud data is untouched).",
                "RESET",
                |b: &mut VaultDriveMockBackend| {
                    *b = VaultDriveMockBackend::default();
                    Ok("Configuration restored to factory defaults".to_string())
                },
            )))
        }),
    ));

    // Tier 2 Universal Add-Ons [19] and [20]
    registry.register_option(AddonOptionEntry::new(1, 0, section_gap_option(19)));
    registry.register_option(AddonOptionEntry::new(1, 0, color_theme_option(20)));

    // Page 2: Diagnostics & Active Transfers
    registry.register_section(AddonSectionEntry::new(
        2,
        0,
        Section::display(Box::new(ActiveTransfersWidget))
            .with_heights(6, 12)
            .with_selectable(false),
    ));
    registry.register_section(AddonSectionEntry::new(
        2,
        1,
        Section::menu("Diagnostic Tools", "1-2 inspect", Vec::new())
            .with_heights(5, 8)
            .with_selectable(true),
    ));
    registry.register_option(AddonOptionEntry::new(
        2,
        1,
        OptionSpec::new(1, "Inspect Systemd Journal", |_ctx| {
            OptionOutcome::PushSubView(Box::new(LiveJournalSubView))
        }),
    ));
    registry.register_option(AddonOptionEntry::new(
        2,
        1,
        OptionSpec::new(2, "Documentation (Clickable Link)", |_ctx| {
            OptionOutcome::Toast {
                message: format_hyperlink(
                    "https://github.com/vaultdrive/tui-rs",
                    "Click: https://github.com/vaultdrive/tui-rs",
                ),
                is_error: false,
            }
        }),
    ));

    if let Ok(qr) = QrWidget::from_text(
        "Mobile Companion Pair",
        "https://github.com/vaultdrive/tui-rs",
    ) {
        registry.register_section(AddonSectionEntry::new(
            2,
            2,
            Section::display(Box::new(qr))
                .with_heights(8, 14)
                .with_selectable(false),
        ));
    }

    DashboardEngine::new(VaultDriveMockBackend::default(), registry.assemble_pages())
}

fn run_headless_check() {
    let mut engine = build_vaultdrive_engine();
    for (cols, rows) in [(80, 24), (120, 32), (180, 44)] {
        let backend = TestBackend::new(TerminalWidth::new(cols), TerminalHeight::new(rows));
        let (w, h) = (backend.dimensions().0, backend.dimensions().1);
        let area = Rect::from_size(w, h);
        let mut buf = Buffer::empty(area);

        // Render Page 1, Page 2, Page 3 and verify no panics or empty buffers across sizes
        for page_idx in 1..=3 {
            let _ = engine.handle_key(KeyPress::new(KeyCode::F(page_idx)));
            engine.tick();
            engine.render(&mut buf, area);
            assert_eq!(buf.to_visual_lines().len(), usize::from(rows));
        }
    }
}

fn main() -> Result<(), tui_rs::error::TuiError> {
    if std::env::args().any(|arg| arg == "--headless-check") {
        run_headless_check();
        return Ok(());
    }

    install_panic_restoration_hook();
    let backend = CrosstermBackend::with_writer(std::io::stdout());
    let is_tty = backend.capabilities().is_tty_console();
    let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    let mut engine = build_vaultdrive_engine().with_tty_mode(is_tty);

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        session.draw(|buffer, area| engine.render(buffer, area))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_ev) => {
                    if let Some(kp) = KeyPress::from_crossterm(key_ev)
                        && engine.handle_key(kp) == EngineAction::Quit
                    {
                        break;
                    }
                }
                crossterm::event::Event::Resize(_, _) => continue,
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            engine.tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}
