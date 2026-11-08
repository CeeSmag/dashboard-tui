# Changelog

All notable changes to `dashboard-tui` are recorded here. Versions follow [SemVer](https://semver.org);
the entry for a version is written in the same commit that raises it (`RULES.md` §8).

## [0.4.0] - 2026-11-08

### Changed
- **Breaking:** the crate is renamed from `tui-rs` to `dashboard-tui` (import path `dashboard_tui`),
  matching the repository rename to `CeeSmag/dashboard-tui`. No API changes otherwise.

## [0.3.0] - 2026-11-08

### Added
- Widget layer: `Widget` / `StatefulWidget` traits with `Text`, `List`, `Table`, `Paragraph`,
  `Tabs`, `Gauge`, `LineGauge`, `Sparkline`, `BarChart`, `Chart`, `Canvas`, `Calendar`, `Clear`
  and a heap-free dot plotter.
- Layout: `Ratio` constraint, `Flex` modes, `areas::<N>`, heap-free `split_into`, `Rect` helpers.
- Core and terminal: `Stylize`, border merging, cursor control, inline viewport.
- Dashboard: sub-pages, settings form, guarded keys, option shortkeys, tab badges, hint flashes,
  log viewer, `StatusPanel` and `ThroughputPanel`, terminal background modes with in-app picker.
- `examples/widget_gallery.rs` and widget-layer documentation.

## [0.2.0] - 2026-11-08

### Changed
- **Breaking:** `EngineAction` (`Continue`, `Back`, `Quit`) is returned by `run_interactive`;
  `q`/cancel at the root returns `Back`, the quit binding returns `Quit`, `Ctrl+C` returns `Back`.
- `ModalResult::Consumed` lets text fields accept characters such as `q`.

## [0.1.0] - 2026-11-07

### Added
- Zero-allocation TUI core, diff-based renderer, and the 3-tier portable dashboard engine.
- QR codes and OSC 8 hyperlinks, developer-defined `Ctrl+[key]` bindings.
