# Engineering, Architectural, and API Design Laws (`RULES.md`)

> **Target Audience: AI Agents & Automated Code Generators**
> This document contains strict, non-negotiable engineering directives for building and maintaining this production-grade Rust Terminal User Interface (TUI) library. Every generated module, type, trait, and test **MUST** comply with these seven pillars before code is committed.

---

## 1. The Laws of Idiomatic Rust & Library Architecture

### 1.1 API Ergonomics (`C-CONV`, `C-GOOD-ERR`, `C-SERDE`)
- Public functions and builders **MUST** accept generic borrowed or convertible types where practical (`impl AsRef<str>`, `Into<Cow<'a, str>>`, `impl IntoIterator`) to eliminate caller boilerplate.
- All pure data types (`Rect`, `Style`, `Color`, `Margin`, domain indices) **MUST** implement `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, and `Default` where semantically valid, and **SHOULD** support optional `serde::Serialize` and `serde::Deserialize` behind a `serde` feature flag.

### 1.2 Type-Driven Safety
- Raw integer primitives (`u16`, `usize`) **MUST NOT** be passed across public API boundaries when representing distinct domain coordinates or dimensions. Use the Newtype pattern (`RowIndex`, `ColumnIndex`, `TerminalWidth`, `TerminalHeight`) to make axis/dimension mix-ups a compile-time error.
- Complex terminal or widget initialization sequences **MUST** use either the **Typestate Pattern** (enforcing valid state transitions at compile time, such as `RawModeDisengaged -> RawModeEngaged`) or a non-consuming/consuming **Builder Pattern**.

### 1.3 Strict Error Handling
- Production library code **MUST NOT** panic.
- `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, and `unimplemented!()` are **STRICTLY FORBIDDEN** outside of `#[cfg(test)]` modules.
- If an invariant is mathematically proven to hold and cannot be expressed through types alone, you **MUST** annotate the exact expression with `#[expect(clippy::unwrap_used, reason = "explicit mathematical proof why this cannot fail")]`.
- Public APIs **MUST** define granular, actionable domain error enums using `thiserror`. Using `anyhow` or `Box<dyn std::error::Error>` in public signatures is **FORBIDDEN**.

### 1.4 Visibility & Encapsulation
- Struct fields **MUST** be private by default. Expose state exclusively through read-only accessor methods (`pub const fn`) or mutation builders to preserve SemVer compatibility.
- Every public error enum and configuration struct **MUST** be annotated with `#[non_exhaustive]` so adding variants or fields in minor releases never breaks downstream callers.

### 1.5 Unsafe Code Policy
- Crate roots **MUST** declare `#![forbid(unsafe_code)]` by default.
- If low-level OS terminal FFI/ioctl calls strictly require `unsafe` in a dedicated backend driver module, the crate root may use `#![deny(unsafe_code)]` and allow it strictly on the isolated submodule, where **every single `unsafe` block MUST** be preceded by a `// SAFETY:` comment proving why all memory and OS invariants hold.

#### Good vs. Bad: Type Safety, Encapsulation & Error Handling

```rust
// ❌ BAD: Raw primitives, public fields, unannotated unwrap, stringly-typed errors
pub struct Viewport {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub title: String,
}

impl Viewport {
    pub fn new(x: u16, y: u16, w: u16, h: u16, title: String) -> Result<Self, String> {
        if w == 0 {
            panic!("width cannot be zero!"); // ❌ Panic in library code
        }
        Ok(Self { x, y, w, h, title })
    }
}

// ✅ GOOD: Newtypes, private fields, Cow ergonomics, structured non-exhaustive errors
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ColumnIndex(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RowIndex(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TerminalWidth(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TerminalHeight(pub u16);

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ViewportError {
    #[error("viewport dimensions must be non-zero (got width={width}, height={height})")]
    ZeroDimensions { width: u16, height: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viewport<'a> {
    origin_column: ColumnIndex,
    origin_row: RowIndex,
    width: TerminalWidth,
    height: TerminalHeight,
    title: Cow<'a, str>,
}

impl<'a> Viewport<'a> {
    pub fn try_new(
        origin_column: ColumnIndex,
        origin_row: RowIndex,
        width: TerminalWidth,
        height: TerminalHeight,
        title: impl Into<Cow<'a, str>>,
    ) -> Result<Self, ViewportError> {
        if width.0 == 0 || height.0 == 0 {
            return Err(ViewportError::ZeroDimensions {
                width: width.0,
                height: height.0,
            });
        }
        Ok(Self {
            origin_column,
            origin_row,
            width,
            height,
            title: title.into(),
        })
    }

    #[must_use]
    pub const fn width(&self) -> TerminalWidth {
        self.width
    }
}
```

---

## 2. Self-Explanatory Naming & Domain Clarity

### 2.1 Unambiguous Identifiers
- Variable, type, and function names **MUST** reveal intent, domain context, and physical units without requiring IDE hover documentation (e.g., `frame_budget_ms`, `viewport_inner_rect`, `cursor_blink_interval_ms`).
- Single-letter variables (`t`, `w`, `h`, `b`) and numbered identifiers (`rect2`, `buf_tmp`) are **FORBIDDEN** outside of 1-line closure predicates.

### 2.2 Rust Conversion Idioms (`C-CONV`)
- Methods performing data conversions **MUST** strictly adhere to ownership and cost prefixes:
  - `as_*`: Free, non-allocating borrowed-to-borrowed view conversion (e.g., `as_cells(&self) -> &[Cell]`, `as_str(&self) -> &str`).
  - `to_*`: Expensive borrowed-to-owned transformation or cloning operation (e.g., `to_ansi_string(&self) -> String`).
  - `into_*`: Consuming owned-to-owned transformation that transfers ownership without re-allocating (e.g., `into_raw_bytes(self) -> Vec<u8>`).

### 2.3 TUI Domain Ubiquitous Language
- All modules **MUST** use the exact canonical domain vocabulary below. Mixing synonyms (e.g., using `Area`, `Box`, `Region`, and `Bounds` interchangeably for `Rect`) is **STRICTLY FORBIDDEN**:
  - **`Screen`**: The physical or virtual terminal display surface managed by the backend driver.
  - **`Viewport`**: A logical rectangular window mapped onto all or part of the `Screen`.
  - **`Rect`**: An axis-aligned rectangle defined by `(ColumnIndex, RowIndex, TerminalWidth, TerminalHeight)`.
  - **`Buffer`**: A 2D contiguous grid of `Cell` elements representing a single renderable surface state.
  - **`Cell`**: A single terminal grid coordinate holding exactly one grapheme cluster, foreground/background `Color`, and text `Modifiers`.
  - **`Span`**: A contiguous inline run of text sharing a single `Style` with no line breaks.
  - **`Line`**: An ordered horizontal sequence of `Span` elements representing a single row of styled text.
  - **`Style`**: Visual attributes combining foreground `Color`, background `Color`, underline `Color`, and bitflag `Modifiers`.
  - **`Layout` & `Constraint`**: The declarative spatial solver (`Constraint::Fixed`, `Constraint::Percentage`, `Constraint::Min`, `Constraint::Fill`) that splits a `Rect` into sub-`Rect`s.

### 2.4 Boolean & Predicate Clarity
- Every method returning `bool` **MUST** be prefixed with `is_`, `has_`, `can_`, or `should_` (e.g., `is_focused()`, `has_dirty_cells()`, `can_scroll_down()`, `should_wrap_graphemes()`).

#### Good vs. Bad: Naming, Conversions & Predicates

```rust
// ❌ BAD: Cryptic abbreviations, mixed terminology ("Box"/"Area"), wrong conversion prefix
impl AreaBox {
    pub fn dirty(&self) -> bool { /* ... */ }
    pub fn as_owned_vec(&self) -> Vec<Cell> { /* allocates! */ }
    pub fn get_t(&self) -> u64 { /* what unit is t? */ }
}

// ✅ GOOD: Ubiquitous language, clear units, strict Rust conversion prefixes
impl Buffer {
    #[must_use]
    pub const fn has_dirty_cells(&self) -> bool { /* ... */ }

    #[must_use]
    pub fn as_cell_slice(&self) -> &[Cell] { /* zero-cost borrow */ }

    #[must_use]
    pub fn to_cloned_cells(&self) -> Vec<Cell> { /* explicit allocation */ }

    #[must_use]
    pub fn into_cells(self) -> Vec<Cell> { /* consuming zero-copy move */ }
}
```

---

## 3. Zero-Allocation Rendering & Memory Discipline (TUI Hot-Path)

### 3.1 Render Loop Purity
- The widget render pass (`Widget::render`) **MUST** be deterministic, side-effect-free, and zero-allocation.
- Heap allocations (`String::new()`, `Vec::new()`, `format!()`, `.to_string()`, `Box::new()`) inside `render()` methods are **FORBIDDEN**. Widgets **MUST** write graphemes and styles directly into pre-allocated `Buffer` cells.

### 3.2 Diff-Based Cell Buffering
- The terminal runtime **MUST** maintain double-buffered `Buffer` instances (`current_buffer` and `previous_buffer`).
- At the end of each frame, the renderer **MUST** compute the minimal delta between `current_buffer` and `previous_buffer` and emit ANSI/CSI escape sequences exclusively for mutated `Cell` coordinates.

### 3.3 Batched I/O Flushing
- The backend **MUST NOT** invoke `Write::flush()` per cell, per line, or per widget.
- All cursor movements, SGR style transitions, and grapheme bytes for a frame diff **MUST** be serialized into a single pre-allocated `Vec<u8>` frame command buffer and flushed to the OS file descriptor in a single atomic write cycle per frame.

#### Good vs. Bad: Zero-Allocation Hot-Path Rendering

```rust
// ❌ BAD: Allocates Strings per frame and flushes stdout inside widget rendering
pub fn draw_status_bar_bad(stdout: &mut std::io::Stdout, username: &str, width: u16) {
    let formatted = format!(" User: {} ", username); // ❌ Heap allocation in hot path
    let padded = format!("{:<width$}", formatted, width = width as usize); // ❌ Another allocation
    let _ = std::io::Write::write_all(stdout, padded.as_bytes());
    let _ = std::io::Write::flush(stdout); // ❌ Per-widget syscall flush causes flicker
}

// ✅ GOOD: Zero-allocation direct Cell buffer mutation; I/O is deferred to frame diff flush
pub fn render_status_bar(
    viewport_rect: Rect,
    buffer: &mut Buffer,
    username: &str,
    base_style: Style,
) {
    buffer.set_style(viewport_rect, base_style);
    let mut cursor_col = viewport_rect.left();
    let max_col = viewport_rect.right();

    cursor_col = buffer.set_str_clipped(cursor_col, viewport_rect.top(), " User: ", max_col, base_style);
    cursor_col = buffer.set_str_clipped(cursor_col, viewport_rect.top(), username, max_col, base_style);
    let _ = buffer.set_str_clipped(cursor_col, viewport_rect.top(), " ", max_col, base_style);
}
```

---

## 4. Terminal State Safety, Resilience & Panic Recovery

### 4.1 RAII Terminal Guards
- Entering raw mode, switching to the alternate screen buffer, enabling mouse event capture, and hiding the terminal cursor **MUST** be encapsulated in an RAII guard struct (`TerminalSessionGuard`).
- The `Drop` implementation of the guard **MUST** unconditionally restore canonical mode, leave the alternate screen, disable mouse capture, and reveal the cursor in reverse acquisition order.

### 4.2 Panic Hook Restoration
- The library **MUST** provide an explicit, composable panic-hook installer (`install_panic_restoration_hook()`) that resets terminal state *before* delegating to the previous panic hook. A panic **MUST NEVER** leave the user's shell in raw mode or trapped on the alternate screen with an invisible cursor.

### 4.3 Signal & Resize Handling
- Event loops and buffer managers **MUST** gracefully handle `SIGWINCH` (resize), `SIGINT`, and `SIGTERM` without corrupting active buffers or deadlocking.
- Every layout calculation and coordinate offset **MUST** guard against zero-width or zero-height dimensions (`width == 0 || height == 0`) and **MUST** use saturating arithmetic (`u16::saturating_add`, `u16::saturating_sub`, `u16::saturating_mul`) to guarantee zero integer underflow/overflow panics.

#### Good vs. Bad: Terminal Lifecycle & Geometry Arithmetic

```rust
// ❌ BAD: Manual teardown (skipped on early `?` or panic) and unchecked subtraction underflow
pub fn run_app_bad(term: &mut RawTerminal) -> Result<(), AppError> {
    term.enable_raw_mode()?;
    term.enter_alt_screen()?;
    let inner_width = term.width() - 2; // ❌ Panics with arithmetic underflow if width < 2!
    do_fallible_work(inner_width)?;     // ❌ Early return leaks raw mode and alt screen!
    term.leave_alt_screen()?;
    term.disable_raw_mode()?;
    Ok(())
}

// ✅ GOOD: RAII Drop guard guarantees cleanup; saturating math prevents underflow on tiny terminals
pub struct TerminalSessionGuard<B: Backend> {
    backend: B,
}

impl<B: Backend> TerminalSessionGuard<B> {
    pub fn enter(mut backend: B) -> Result<Self, B::Error> {
        backend.enable_raw_mode()?;
        if let Err(err) = backend.enter_alternate_screen() {
            let _ = backend.disable_raw_mode();
            return Err(err);
        }
        Ok(Self { backend })
    }

    #[must_use]
    pub fn inner_border_width(&self) -> TerminalWidth {
        // Safe even if terminal is resized to 0x0 or 1x1
        TerminalWidth(self.backend.dimensions().width.0.saturating_sub(2))
    }
}

impl<B: Backend> Drop for TerminalSessionGuard<B> {
    fn drop(&mut self) {
        let _ = self.backend.show_cursor();
        let _ = self.backend.leave_alternate_screen();
        let _ = self.backend.disable_raw_mode();
        let _ = self.backend.flush();
    }
}
```

---

## 5. Unicode, Grapheme & Cross-Platform Correctness

### 5.1 Grapheme Cluster Awareness
- Code **MUST NOT** index, slice (`&text[0..n]`), or measure user-facing strings by byte length (`str::len()`) or scalar `char` count (`str::chars().count()`) for layout math.
- All text measurement, truncation, cursor stepping, and line wrapping **MUST** segment strings by Unicode Standard Annex #29 extended grapheme clusters (`unicode_segmentation::UnicodeSegmentation::graphemes(true)`) and compute visual column occupation via East Asian display widths (`unicode_width::UnicodeWidthStr::width`).
- Multi-column graphemes (e.g., 2-column CJK ideographs or emojis) that land on the final single column of a `Rect` **MUST** be replaced with a space padding glyph rather than overflowing the right border.

### 5.2 Color & Capability Degradation
- The styling engine **MUST** support `Color::Rgb(u8, u8, u8)` (24-bit TrueColor), `Color::Indexed(u8)` (256-color), 16-color ANSI, and `Color::Reset`.
- Backends **MUST** respect the `NO_COLOR` environment variable and automatically down-sample TrueColor to 256-color or 16-color palettes when terminal capabilities dictate.

### 5.3 Backend Agnosticism
- Widget rendering, layout solving, and state management **MUST** have zero compile-time dependency on concrete terminal drivers (`crossterm`, `termion`, `libc`).
- All hardware interaction **MUST** pass through the abstract `Backend` trait so applications can swap drivers or run deterministically against an in-memory `TestBackend`.

#### Good vs. Bad: Unicode Grapheme Measurement & Clipping

```rust
// ❌ BAD: Byte slicing panics on multi-byte UTF-8; char count misaligns CJK/emoji widths
pub fn truncate_line_bad(text: &str, max_columns: usize) -> &str {
    if text.len() > max_columns {
        &text[..max_columns] // ❌ Panics at runtime if max_columns splits a UTF-8 sequence!
    } else {
        text
    }
}

// ✅ GOOD: UAX #29 grapheme segmentation with East Asian column width accounting
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[must_use]
pub fn clip_to_terminal_width(text: &str, max_width: TerminalWidth) -> &str {
    let limit = usize::from(max_width.0);
    let mut accumulated_columns = 0usize;
    let mut byte_end = 0usize;

    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme.width();
        if accumulated_columns.saturating_add(grapheme_width) > limit {
            break;
        }
        accumulated_columns = accumulated_columns.saturating_add(grapheme_width);
        byte_end = byte_end.saturating_add(grapheme.len());
    }

    &text[..byte_end]
}
```

---

## 6. Testing, Documentation & Static Analysis Standards

### 6.1 Headless Snapshot Testing
- Every widget and layout component **MUST** be verifiable without a physical TTY by rendering into an in-memory `TestBackend` and asserting against human-readable visual text grids (e.g., via `insta::assert_snapshot!`).

### 6.2 Property-Based Layout Testing
- All layout solvers, constraint splitters, and text wrappers **MUST** include property-based test suites (`proptest`) verifying that arbitrary terminal dimensions across `0..=u16::MAX` never panic, overflow, or produce child `Rect`s extending outside parent bounds.

### 6.3 Documentation as a Gate
- Every crate root **MUST** enforce `#![deny(missing_docs)]` and `#![deny(rustdoc::broken_intra_doc_links)]`.
- Every public struct, enum, trait, and method **MUST** have a rustdoc comment containing:
  - A concise summary sentence.
  - A compiling `# Examples` block demonstrating realistic usage.
  - `# Errors` (if returning `Result`) detailing exact failure conditions.

### 6.4 Required `Cargo.toml` Lints Configuration
Every crate in the workspace **MUST** include the following `[lints]` block in `Cargo.toml`:

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "deny"
missing_debug_implementations = "deny"
rust_2018_idioms = { level = "deny", priority = -1 }
unreachable_pub = "deny"

[lints.rustdoc]
broken_intra_doc_links = "deny"
missing_crate_level_docs = "deny"

[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }

# Strict safety & production reliability gates
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
undocumented_unsafe_blocks = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
arithmetic_side_effects = "warn"
cast_possible_truncation = "deny"
cast_sign_loss = "deny"
must_use_candidate = "deny"
```

#### Good vs. Bad: Headless & Property-Based Testing

```rust
// ❌ BAD: Untestable widget coupled to physical stdout; only checks a single hardcoded size
#[test]
fn test_widget_bad() {
    let w = MyWidget::new("Hello");
    assert_eq!(w.title, "Hello"); // Does not test layout math or cell rendering at all!
}

// ✅ GOOD: Headless visual grid assertion + Proptest invariant fuzzing across 0..=u16::MAX
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn renders_bordered_panel_within_8x3_test_backend() {
        let rect = Rect::new(ColumnIndex(0), RowIndex(0), TerminalWidth(8), TerminalHeight(3));
        let mut buffer = Buffer::empty(rect);
        let panel = Panel::new("Vault");

        panel.render(rect, &mut buffer);

        assert_eq!(
            buffer.to_debug_grid_lines(),
            vec![
                "┌Vault─┐",
                "│      │",
                "└──────┘",
            ]
        );
    }

    proptest! {
        #[test]
        fn layout_solver_never_exceeds_parent_bounds(
            x in 0..=u16::MAX,
            y in 0..=u16::MAX,
            width in 0..=u16::MAX,
            height in 0..=u16::MAX,
            split_pct in 0u16..=100u16,
        ) {
            let parent = Rect::new(
                ColumnIndex(x),
                RowIndex(y),
                TerminalWidth(width),
                TerminalHeight(height),
            );
            let chunks = Layout::horizontal([
                Constraint::Percentage(split_pct),
                Constraint::Fill(1),
            ])
            .split(parent);

            for chunk in chunks {
                prop_assert!(chunk.width().0 <= parent.width().0);
                prop_assert!(chunk.height().0 <= parent.height().0);
            }
        }
    }
}
```

---

## 7. Modularity, Decoupling & Anti-God-File Laws

### 7.1 No God Files (600-Line Ceiling)
- No single `.rs` source file **MAY** exceed **600 lines of production code** (excluding `#[cfg(test)]` modules and rustdoc comments).
- Before a module approaches 600 lines, it **MUST** be refactored into cohesive submodules separated by concern (e.g., `table/mod.rs`, `table/state.rs`, `table/layout.rs`, `table/render.rs`).

### 7.2 Single Responsibility Per Module
- Every `.rs` file **MUST** encapsulate a single cohesive responsibility.
- Mixing OS terminal I/O, constraint layout solving, and stateful widget logic inside the same file is **STRICTLY FORBIDDEN**.

### 7.3 Unidirectional Dependency Graph
- Module dependencies **MUST** flow strictly downward in a Directed Acyclic Graph (DAG):
  ```text
  Application / Stateful Components
                │
                ▼
      Widgets (Stateless & Stateful)
                │
                ▼
  Core Primitives (Buffer, Cell, Rect, Style, Layout, Text)
                │
                ▼
       Backend Trait Boundary
  ```
- **Inversion Law**: Core primitives (`buffer`, `layout`, `style`, `text`, `geometry`) **MUST NEVER** import from `widgets` or concrete backend driver implementations.

### 7.4 Deep Implementation, Flat Public Facade
- Internal implementation files **MUST** be organized into focused submodule trees using `pub(crate)` visibility for internal helpers.
- Public types **MUST** be re-exported cleanly via `pub use` at top-level semantic module boundaries (`crate::prelude`, `crate::widgets`, `crate::layout`, `crate::style`, `crate::backend`, `crate::terminal`) so library consumers never import deep internal file paths.

#### Good vs. Bad: Module Organization & Public Facade

```rust
// ❌ BAD: Deep leaky module tree forcing callers to know internal file layout, plus circular imports
// Caller forced to write:
// use my_tui::widgets::containers::scrollable::table::render_engine::TableWidget;
// And inside `core/buffer.rs`:
use crate::widgets::table::TableState; // ❌ Forbidden upward dependency from Core -> Widgets!

// ✅ GOOD: Clean decomposed submodules with a flat top-level facade (`src/widgets/mod.rs`)
mod block;
mod list;
mod table {
    pub(crate) mod column_solver;
    pub(crate) mod render;
    pub(crate) mod state;
    pub(crate) mod widget;
}

// Flat re-exports at the `widgets` module boundary:
pub use self::block::{Block, BorderType, Borders};
pub use self::list::{List, ListItem, ListState};
pub use self::table::{
    state::TableState,
    widget::{Row, Table},
};
```

---

## 8. Release & Versioning Law (`REL`)

### 8.1 Every Push Raises the Version (`REL-1`)
- Every push to `origin` that changes any file **MUST** raise `version` in `Cargo.toml`. Two pushes never share a version number, so a version always identifies exactly one state of the code.
- Follow SemVer: breaking public API change → major (minor while `0.x`); new backward-compatible feature → minor (patch while `0.x`); fix, docs or internal change → patch.
- `Cargo.lock` **MUST** be updated in the same commit.

### 8.2 Changelog Entry (`REL-2`)
- The commit that raises the version **MUST** add a `## [X.Y.Z] - YYYY-MM-DD` section to `CHANGELOG.md` describing what changed. A version without an entry is invalid.

### 8.3 Tag Every Version (`REL-3`)
- After the push, the version **MUST** be tagged `vX.Y.Z` (`git tag vX.Y.Z && git push --tags`) so consumers can pin `tag = "vX.Y.Z"`. Tags are never moved or deleted.

### 8.4 Mechanical Enforcement (`REL-4`)
- `scripts/check-version-bump.sh` verifies `REL-1` and `REL-2` against the previous state of the branch; CI runs it on every push and pull request. Run it locally before pushing: `scripts/check-version-bump.sh origin/main`.
