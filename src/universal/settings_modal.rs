//! Universal multi-field settings modal (`SettingsModal`).
//!
//! A data-driven form: the developer lists fields (choice, toggle, number) with a getter
//! and setter each, and the modal handles selection, live editing, cancel-with-revert and
//! saving. Changes apply to the backend immediately so the dashboard previews them; `Esc` /
//! `q` restores every original value, `Enter` keeps them and runs the optional save hook.

use core::fmt::Write;

use crate::components::SliderBar;
use crate::core::buffer::{Buffer, StackStr};
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::dashboard::backend_trait::DashboardBackend;
use crate::dashboard::key_router::{KeyCode, KeyPress};
use crate::dashboard::modal::{Modal, ModalContext, ModalResult};
use crate::widgets::choice_row::ChoiceRow;

/// Reads a field's current value from the backend.
pub type SettingGetFn<B> = Box<dyn Fn(&B) -> i32>;

/// Writes a new value into the backend.
pub type SettingSetFn<B> = Box<dyn Fn(&mut B, i32)>;

/// Persists the settings after `Enter`; `Ok(message)` shows a success toast.
pub type SettingsSaveFn<B> = Box<dyn FnMut(&mut B) -> Result<String, String>>;

/// How a [`SettingField`] is edited.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SettingKind {
    /// One of several named options; the value is the option index.
    Choice(Vec<String>),
    /// On / off; the value is `0` or `1`.
    Toggle,
    /// An integer between `min` and `max` moved by `step`.
    Number { min: i32, max: i32, step: i32 },
}

/// One editable row of a [`SettingsModal`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::DashboardBackend;
/// use dashboard_tui::universal::SettingField;
///
/// struct App { workers: i32, hw_accel: bool }
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let workers = SettingField::<App>::number("Workers", 1, 16, |a| a.workers, |a, v| a.workers = v);
/// let accel = SettingField::<App>::toggle("Hardware accel", |a| a.hw_accel, |a, on| a.hw_accel = on);
/// assert_eq!(workers.label(), "Workers");
/// assert_eq!(accel.label(), "Hardware accel");
/// ```
pub struct SettingField<B: DashboardBackend> {
    label: String,
    detail: String,
    kind: SettingKind,
    get: SettingGetFn<B>,
    set: SettingSetFn<B>,
}

impl<B: DashboardBackend> core::fmt::Debug for SettingField<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SettingField")
            .field("label", &self.label)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> SettingField<B> {
    /// A field choosing one of `options`; getter and setter use the option index.
    #[must_use]
    pub fn choice<I, S>(
        label: impl Into<String>,
        options: I,
        get: impl Fn(&B) -> usize + 'static,
        set: impl Fn(&mut B, usize) + 'static,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            label: label.into(),
            detail: String::new(),
            kind: SettingKind::Choice(options.into_iter().map(Into::into).collect()),
            get: Box::new(move |backend| i32::try_from(get(backend)).unwrap_or(0)),
            set: Box::new(move |backend, value| set(backend, usize::try_from(value).unwrap_or(0))),
        }
    }

    /// An on/off field.
    #[must_use]
    pub fn toggle(
        label: impl Into<String>,
        get: impl Fn(&B) -> bool + 'static,
        set: impl Fn(&mut B, bool) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            detail: String::new(),
            kind: SettingKind::Toggle,
            get: Box::new(move |backend| i32::from(get(backend))),
            set: Box::new(move |backend, value| set(backend, value != 0)),
        }
    }

    /// An integer field between `min` and `max` (step 1; see [`SettingField::with_step`]).
    #[must_use]
    pub fn number(
        label: impl Into<String>,
        min: i32,
        max: i32,
        get: impl Fn(&B) -> i32 + 'static,
        set: impl Fn(&mut B, i32) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            detail: String::new(),
            kind: SettingKind::Number {
                min: min.min(max),
                max: max.max(min),
                step: 1,
            },
            get: Box::new(get),
            set: Box::new(set),
        }
    }

    /// Sets how far `←` / `→` move a number field (ignored for other kinds).
    #[must_use]
    pub fn with_step(mut self, step: i32) -> Self {
        if let SettingKind::Number { step: current, .. } = &mut self.kind {
            *current = step.max(1);
        }
        self
    }

    /// Sets the dim help text shown under the row.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Returns the row label.
    #[must_use]
    #[inline]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the value reached by moving `direction` (`-1` / `+1`) from `current`.
    fn stepped(&self, current: i32, direction: i32) -> i32 {
        match &self.kind {
            SettingKind::Choice(options) => {
                let count = i32::try_from(options.len()).unwrap_or(i32::MAX).max(1);
                current.saturating_add(direction).rem_euclid(count)
            }
            SettingKind::Toggle => i32::from(current == 0),
            SettingKind::Number { min, max, step } => current
                .saturating_add(direction.saturating_mul(*step))
                .clamp(*min, *max),
        }
    }
}

/// Multi-field settings form with live preview, revert-on-cancel, and an optional save hook.
///
/// Keys: `↑↓`/`j k` select a field, `←→`/`h l` change it (`Space` toggles), `Enter` saves,
/// `Esc` / `q` cancels and restores the original values.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Color, PrioritySegment};
/// use dashboard_tui::dashboard::DashboardBackend;
/// use dashboard_tui::universal::SettingsModal;
///
/// struct App { workers: i32, quality: usize }
/// impl DashboardBackend for App {
///     fn app_title(&self) -> &str { "App" }
///     fn status_badge(&self) -> (&str, Color) { ("OK", Color::Green) }
///     fn header_segments(&self) -> Vec<PrioritySegment> { Vec::new() }
/// }
///
/// let modal = SettingsModal::<App>::new("Transfer Settings")
///     .with_number("Workers", 1..=16, |a| a.workers, |a, v| a.workers = v)
///     .with_choice("Quality", ["Low", "Medium", "High"], |a| a.quality, |a, i| a.quality = i)
///     .on_save(|_app| Ok("Settings saved".to_string()));
/// assert_eq!(modal.field_count(), 2);
/// ```
pub struct SettingsModal<B: DashboardBackend> {
    title: String,
    fields: Vec<SettingField<B>>,
    cursor: usize,
    originals: Option<Vec<i32>>,
    on_save: Option<SettingsSaveFn<B>>,
}

impl<B: DashboardBackend> core::fmt::Debug for SettingsModal<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SettingsModal")
            .field("title", &self.title)
            .field("fields_len", &self.fields.len())
            .field("cursor", &self.cursor)
            .finish_non_exhaustive()
    }
}

impl<B: DashboardBackend> SettingsModal<B> {
    /// Creates an empty settings modal titled `title`.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            fields: Vec::new(),
            cursor: 0,
            originals: None,
            on_save: None,
        }
    }

    /// Appends a field.
    #[must_use]
    pub fn with_field(mut self, field: SettingField<B>) -> Self {
        self.fields.push(field);
        self
    }

    /// Appends an integer field between `min` and `max`.
    #[must_use]
    pub fn with_number(
        self,
        label: impl Into<String>,
        range: core::ops::RangeInclusive<i32>,
        get: impl Fn(&B) -> i32 + 'static,
        set: impl Fn(&mut B, i32) + 'static,
    ) -> Self {
        self.with_field(SettingField::number(
            label,
            *range.start(),
            *range.end(),
            get,
            set,
        ))
    }

    /// Appends an on/off field.
    #[must_use]
    pub fn with_toggle(
        self,
        label: impl Into<String>,
        get: impl Fn(&B) -> bool + 'static,
        set: impl Fn(&mut B, bool) + 'static,
    ) -> Self {
        self.with_field(SettingField::toggle(label, get, set))
    }

    /// Appends a field choosing one of `options` (getter/setter use the option index).
    #[must_use]
    pub fn with_choice<I, S>(
        self,
        label: impl Into<String>,
        options: I,
        get: impl Fn(&B) -> usize + 'static,
        set: impl Fn(&mut B, usize) + 'static,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.with_field(SettingField::choice(label, options, get, set))
    }

    /// Sets the hook run when the user presses `Enter` (for example, writing a config file).
    #[must_use]
    pub fn on_save<F>(mut self, save: F) -> Self
    where
        F: FnMut(&mut B) -> Result<String, String> + 'static,
    {
        self.on_save = Some(Box::new(save));
        self
    }

    /// Returns the number of fields.
    #[must_use]
    #[inline]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Remembers the values at first key press so cancelling can restore them.
    fn capture_originals(&mut self, backend: &B) {
        if self.originals.is_none() {
            self.originals = Some(self.fields.iter().map(|f| (f.get)(backend)).collect());
        }
    }

    fn revert(&self, backend: &mut B) {
        if let Some(originals) = &self.originals {
            for (field, original) in self.fields.iter().zip(originals) {
                (field.set)(backend, *original);
            }
        }
    }

    fn change_selected(&mut self, direction: i32, backend: &mut B) {
        self.capture_originals(backend);
        if let Some(field) = self.fields.get(self.cursor) {
            let next = field.stepped((field.get)(backend), direction);
            (field.set)(backend, next);
        }
    }
}

impl<B: DashboardBackend> Modal<B> for SettingsModal<B> {
    fn title(&self) -> &str {
        &self.title
    }

    fn subtitle(&self) -> &'static str {
        "↑↓ select • ←→ change • Enter save • Esc cancel"
    }

    fn desired_size(&self, viewport: Rect) -> (TerminalWidth, TerminalHeight) {
        let width = (viewport.width().get() * 3 / 4).clamp(46, 72);
        let rows = u16::try_from(self.fields.len())
            .unwrap_or(u16::MAX)
            .saturating_mul(2)
            .saturating_add(2);
        (TerminalWidth::new(width), TerminalHeight::new(rows.max(6)))
    }

    fn render(&self, _buffer: &mut Buffer, _inner: Rect, _palette: &ColorPalette, _tty_mode: bool) {
        // The body shows live backend values; see `render_with_backend`.
    }

    fn render_with_backend(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        self.render_fields(backend, buffer, inner, palette, tty_mode);
    }

    fn on_key(&mut self, key: KeyPress, ctx: &mut ModalContext<'_, B>) -> ModalResult {
        let keys = ctx.keys;
        match key.code() {
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                self.revert(ctx.backend);
                ModalResult::Close
            }
            KeyCode::Enter => match self.on_save.as_mut() {
                Some(save) => match save(ctx.backend) {
                    Ok(message) if message.is_empty() => ModalResult::Close,
                    Ok(message) => ModalResult::CloseWithToast {
                        message,
                        is_error: false,
                    },
                    Err(message) => {
                        self.revert(ctx.backend);
                        ModalResult::CloseWithToast {
                            message,
                            is_error: true,
                        }
                    }
                },
                None => ModalResult::Close,
            },
            KeyCode::Left | KeyCode::Char('h') => {
                self.change_selected(-1, ctx.backend);
                ModalResult::Continue
            }
            KeyCode::Right | KeyCode::Char('l' | ' ') => {
                self.change_selected(1, ctx.backend);
                ModalResult::Continue
            }
            _ if keys.is_up(key) => {
                self.cursor = self.cursor.saturating_sub(1);
                ModalResult::Continue
            }
            _ if keys.is_down(key) => {
                self.cursor = self
                    .cursor
                    .saturating_add(1)
                    .min(self.fields.len().saturating_sub(1));
                ModalResult::Continue
            }
            _ => ModalResult::Continue,
        }
    }
}

impl<B: DashboardBackend> SettingsModal<B> {
    fn render_fields(
        &self,
        backend: &B,
        buffer: &mut Buffer,
        inner: Rect,
        palette: &ColorPalette,
        tty_mode: bool,
    ) {
        let mut row = inner.y();
        for (index, field) in self.fields.iter().enumerate() {
            if row.get().saturating_add(1) > inner.bottom().get() {
                break;
            }
            let value = (field.get)(backend);
            let mut badge = StackStr::<32>::new();
            match &field.kind {
                SettingKind::Choice(options) => {
                    let text = usize::try_from(value)
                        .ok()
                        .and_then(|i| options.get(i))
                        .map_or("?", String::as_str);
                    badge.push_str(text);
                }
                SettingKind::Toggle => {
                    badge.push_str(if value != 0 { "On" } else { "Off" });
                }
                SettingKind::Number { max, .. } => {
                    let _ = write!(badge, "{value} / {max}");
                }
            }
            let number = u16::try_from(index.saturating_add(1)).unwrap_or(u16::MAX);
            let used = ChoiceRow::new(number, &field.label)
                .with_badge(badge.as_str())
                .with_badge_color(palette.accent)
                .with_detail(&field.detail)
                .with_selected(index == self.cursor)
                .with_tty_mode(tty_mode)
                .render(buffer, inner.x(), row, inner.width().get(), palette);
            row = row.saturating_add(used);

            if let SettingKind::Number { min, max, .. } = &field.kind {
                if row.get() < inner.bottom().get() && inner.width().get() > 8 {
                    let _ = SliderBar::new(value.clamp(*min, *max), *min, *max)
                        .with_tty_mode(tty_mode)
                        .render(
                            buffer,
                            inner.x().saturating_add(6),
                            row,
                            inner.width().get().saturating_sub(6),
                            palette,
                        );
                    row = row.saturating_add(1);
                }
            }
        }
    }
}
