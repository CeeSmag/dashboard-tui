//! A complete second dashboard, for a completely different application (a web server),
//! written to show how little code the library needs (`examples/webserver_dashboard.rs`).
//!
//! Nothing here is specific to any one product: swap the backend struct and the labels
//! and the same ~120 lines describe any service. It uses only the ready-made universal
//! building blocks:
//! - [`StatusPanel`] / [`ThroughputPanel`] for live telemetry,
//! - [`SettingsModal`] for a multi-field form with live preview,
//! - [`LogViewSubView`] fed by a [`LogBuffer`],
//! - [`PageSubView`] for a drill-down menu page,
//! - a tab badge, a guarded `ctrl+x` key and an option shortkey.
//!
//! ```bash
//! cargo run --example webserver_dashboard
//! cargo run --example webserver_dashboard -- --headless-check
//! ```

use std::io::Write;
use std::time::Duration;

use dashboard_tui::backend::{Backend, TestBackend};
use dashboard_tui::core::{
    Buffer, Color, PrioritySegment, Rect, Style, TerminalHeight, TerminalWidth, diff_buffers,
};
use dashboard_tui::dashboard::{
    AddonRegistry, DashboardBackend, DashboardEngine, KeyBindings, OptionOutcome, OptionSpec,
    PageSubView, Section, SectionLayoutMode,
};
use dashboard_tui::prelude::{LogBuffer, LogViewSubView, RateHistory, SettingsModal};
use dashboard_tui::universal::{ConfirmModal, StatusPanel, ThroughputPanel};

/// The only application-specific part: plain data the dashboard reads.
struct WebServer {
    running: bool,
    cpu_pct: f64,
    memory_pct: f64,
    workers: i32,
    tls_enabled: bool,
    log_level: usize,
    requests: RateHistory,
    responses: RateHistory,
    log: LogBuffer,
    tick: u32,
}

impl WebServer {
    fn new() -> Self {
        let mut log = LogBuffer::new(500);
        log.info("boot", "listening on 0.0.0.0:8080");
        Self {
            running: true,
            cpu_pct: 12.0,
            memory_pct: 38.0,
            workers: 4,
            tls_enabled: true,
            log_level: 1,
            requests: RateHistory::new(120),
            responses: RateHistory::new(120),
            log,
            tick: 0,
        }
    }
}

impl DashboardBackend for WebServer {
    fn app_title(&self) -> &'static str {
        "WebServer"
    }

    fn status_badge(&self) -> (&str, Color) {
        if self.running {
            ("● RUNNING", Color::Green)
        } else {
            ("○ STOPPED", Color::Red)
        }
    }

    fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
        vec![PrioritySegment::new(
            format!("{} workers", self.workers),
            Style::DEFAULT,
            60,
        )]
    }

    fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let wave = f64::from(self.tick % 40);
        self.cpu_pct = 10.0 + wave;
        self.requests.push(200_000.0 + wave * 12_000.0);
        self.responses.push(900_000.0 + wave * 40_000.0);
        if self.tick % 25 == 0 {
            self.log.warn("http", "slow response on /api/export");
        }
    }
}

type Registry = AddonRegistry<WebServer>;

/// Page 1: live telemetry panels.
fn add_overview_page(registry: Registry) -> Registry {
    registry
        .with_page(0, "Overview", "F1", SectionLayoutMode::Auto)
        .with_display_section(
            0,
            0,
            StatusPanel::<WebServer>::new("Server")
                .with_subtitle("web.service")
                .with_badge(|s| {
                    if s.running {
                        ("● ONLINE", Some(Color::Green))
                    } else {
                        ("● OFFLINE", Some(Color::Red))
                    }
                })
                .with_meter("CPU", |s| s.cpu_pct)
                .with_meter("Memory", |s| s.memory_pct)
                .with_rates("Traffic", |s| (s.requests.latest(), s.responses.latest())),
            6,
            8,
            false,
        )
        .with_display_section(
            0,
            1,
            ThroughputPanel::<WebServer>::dual(
                "Throughput",
                "Requests",
                |s| s.requests.as_slice(),
                "Responses",
                |s| s.responses.as_slice(),
            ),
            7,
            12,
            false,
        )
}

/// Page 2: a menu of actions, each opening a modal, a log view, or a drill-down page.
fn add_control_page(registry: Registry) -> Registry {
    registry
        .with_page(1, "Control", "F2", SectionLayoutMode::Auto)
        .with_menu_section(1, 0, "Actions", "1-4 run", 6, 8, true)
        .with_option(
            1,
            0,
            OptionSpec::new(1, "Settings", |_ctx| {
                OptionOutcome::OpenModal(Box::new(
                    SettingsModal::<WebServer>::new("Server Settings")
                        .with_number("Workers", 1..=32, |s| s.workers, |s, v| s.workers = v)
                        .with_toggle("TLS", |s| s.tls_enabled, |s, on| s.tls_enabled = on)
                        .with_choice(
                            "Log level",
                            ["Debug", "Info", "Warn"],
                            |s| s.log_level,
                            |s, i| s.log_level = i,
                        )
                        .on_save(|_s| Ok("Settings saved".to_string())),
                ))
            })
            .with_shortkey('s'),
        )
        .with_option(
            1,
            0,
            OptionSpec::new(2, "View Logs", |_ctx| {
                OptionOutcome::PushSubView(Box::new(LogViewSubView::new(
                    "Logs",
                    |s: &WebServer| s.log.as_slice(),
                )))
            })
            .with_shortkey('l'),
        )
        .with_option(
            1,
            0,
            OptionSpec::new(3, "Network", |_ctx| {
                let mut net = Section::menu("Network", "1 run", Vec::new()).with_heights(5, 8);
                net.push_option(OptionSpec::new(1, "Show listen address", |_ctx| {
                    OptionOutcome::Toast {
                        message: "0.0.0.0:8080".to_string(),
                        is_error: false,
                    }
                }));
                OptionOutcome::PushSubView(Box::new(PageSubView::new("Network").with_section(net)))
            }),
        )
        .with_option(
            1,
            0,
            OptionSpec::new(4, "Restart", |_ctx| {
                OptionOutcome::OpenModal(Box::new(ConfirmModal::warning(
                    "Restart",
                    "Drop all connections and restart?",
                    |s: &mut WebServer| {
                        s.log.info("ctl", "restarted");
                        Ok("Restarted".to_string())
                    },
                )))
            }),
        )
}

fn build_engine() -> DashboardEngine<WebServer> {
    let registry =
        Registry::new().with_keys(KeyBindings::default().quit("ctrl+q").confirm_quit(true));
    add_control_page(add_overview_page(registry))
        // The "Control" tab shows the number of warnings or errors
        .with_tab_badge(1, |s| match s.log.len() {
            0 => String::new(),
            n => format!("{n} log"),
        })
        // A guarded key: ctrl+x warns first, Enter confirms, q cancels
        .bind_confirmed_key(
            "ctrl+x",
            "Clear Logs",
            "Delete all log lines?",
            |s: &mut WebServer| {
                s.log = LogBuffer::new(500);
                Ok("Logs cleared".to_string())
            },
        )
        .build_engine(WebServer::new())
}

fn run_headless_check() -> Result<(), dashboard_tui::error::TuiError> {
    let mut engine = build_engine();
    for _ in 0..30 {
        engine.tick();
    }
    let mut backend = TestBackend::new(TerminalWidth::new(100), TerminalHeight::new(24));
    let viewport = Rect::from_size(TerminalWidth::new(100), TerminalHeight::new(24));
    let mut frame = Buffer::empty(viewport);
    engine.render(&mut frame, viewport);
    backend.draw_frame(diff_buffers(&Buffer::empty(Rect::ZERO), &frame))?;
    backend.flush()?;
    // `KeyHint` is part of the same prelude surface; keep the import exercised.
    writeln!(std::io::stdout(), "{}", backend.to_visual_grid())?;
    Ok(())
}

fn main() -> Result<(), dashboard_tui::error::TuiError> {
    if std::env::args().any(|arg| arg == "--headless-check") {
        return run_headless_check();
    }
    build_engine().run_interactive(Duration::from_millis(250))?;
    Ok(())
}
