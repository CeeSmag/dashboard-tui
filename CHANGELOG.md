# Changelog

All notable changes to `dashboard-tui` are recorded here. Versions follow [SemVer](https://semver.org);
the entry for a version is written in the same commit that raises it (`RULES.md` §8).

## [0.4.3] - 2026-11-08

### Fixed
- `vaultdrive_dashboard` example and `AI_GUIDE.md`: `WaveScaler` is fed only the last ~20 samples
  (~5 s) of each series while `DualWaveform` still draws the full 60 s window, so the scale follows
  recent activity. After a 15 MB/s burst the wave recovers about 6 s after the burst ends instead of
  staying squashed for ~60 s. A regression test simulates 100 KB/s -> 15 MB/s -> 100 KB/s.
- Rustdoc: the `EngineAction::Back` link in `KeyBindings::confirm_quit` docs resolves again after the
  `key_router` split.

## [0.4.2] - 2026-11-08

### Changed
- Internal: `dashboard::key_router` is split into `events`, `key_code`, `key_press` and
  `bindings` submodules (public paths unchanged).

### Fixed
- `vaultdrive_dashboard` example and `AI_GUIDE.md` fed `WaveScaler` only the newest sample; they now
  call `update_sync` with the whole visible window of both series, so the ceiling no longer decays
  while old peaks are still on screen.

## [0.4.1] - 2026-11-08

### Added
- `KeyBindings::quit_popup(title, description)`: the developer chooses the text of the `Ctrl+Q`
  quit warning popup (accessors `quit_popup_title` / `quit_popup_description`).

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
