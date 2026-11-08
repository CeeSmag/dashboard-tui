//! RAII terminal session guard and double-buffered frame runtime.
//!
//! Ensures raw mode, the alternate screen buffer, cursor visibility, and
//! DEC Mode 1007 alternate scroll are unconditionally restored when the
//! session guard is dropped.

use crate::backend::Backend;
use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
use crate::core::diff::diff_buffers;
use crate::core::rect::Rect;
use crate::error::TuiError;

/// Where the UI is drawn on the terminal.
///
/// # Examples
///
/// ```
/// use dashboard_tui::terminal::{TerminalSessionConfig, Viewport};
///
/// let config = TerminalSessionConfig::new().with_viewport(Viewport::Inline(8));
/// assert_eq!(config.viewport(), Viewport::Inline(8));
/// assert!(!config.should_use_alternate_screen());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Viewport {
    /// Take over the whole screen (the default).
    #[default]
    Fullscreen,
    /// Draw in a fixed number of rows at the bottom of the terminal, leaving earlier output
    /// (command history) visible above and untouched.
    Inline(u16),
}

/// Configuration options for a managed terminal session.
///
/// # Examples
///
/// ```
/// use dashboard_tui::terminal::TerminalSessionConfig;
///
/// let config = TerminalSessionConfig::default()
///     .with_alternate_screen(true)
///     .with_hide_cursor(true);
/// assert!(config.should_use_alternate_screen());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TerminalSessionConfig {
    use_alternate_screen: bool,
    hide_cursor: bool,
    enable_alternate_scroll: bool,
    viewport: Viewport,
}

impl Default for TerminalSessionConfig {
    #[inline]
    fn default() -> Self {
        Self {
            use_alternate_screen: true,
            hide_cursor: true,
            enable_alternate_scroll: true,
            viewport: Viewport::Fullscreen,
        }
    }
}

impl TerminalSessionConfig {
    /// Creates a new default session configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// let config = TerminalSessionConfig::new();
    /// assert!(config.should_hide_cursor());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            use_alternate_screen: true,
            hide_cursor: true,
            enable_alternate_scroll: true,
            viewport: Viewport::Fullscreen,
        }
    }

    /// Configures whether the session enters the alternate screen buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// let config = TerminalSessionConfig::new().with_alternate_screen(false);
    /// assert!(!config.should_use_alternate_screen());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_alternate_screen(mut self, enabled: bool) -> Self {
        self.use_alternate_screen = enabled;
        self
    }

    /// Configures whether the cursor is hidden while the session is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// let config = TerminalSessionConfig::new().with_hide_cursor(false);
    /// assert!(!config.should_hide_cursor());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_hide_cursor(mut self, enabled: bool) -> Self {
        self.hide_cursor = enabled;
        self
    }

    /// Configures whether DEC Mode 1007 (mouse wheel to Up/Down arrows) is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// let config = TerminalSessionConfig::new().with_alternate_scroll(false);
    /// assert!(!config.should_enable_alternate_scroll());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_alternate_scroll(mut self, enabled: bool) -> Self {
        self.enable_alternate_scroll = enabled;
        self
    }

    /// Chooses where the UI is drawn. [`Viewport::Inline`] turns the alternate screen off, so
    /// the UI lives in the last rows of the normal terminal and earlier output stays visible.
    #[must_use]
    #[inline]
    pub const fn with_viewport(mut self, viewport: Viewport) -> Self {
        if matches!(viewport, Viewport::Inline(_)) {
            self.use_alternate_screen = false;
        }
        self.viewport = viewport;
        self
    }

    /// Returns the configured [`Viewport`].
    #[must_use]
    #[inline]
    pub const fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// Returns `true` if the session uses the alternate screen buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// assert!(TerminalSessionConfig::default().should_use_alternate_screen());
    /// ```
    #[must_use]
    #[inline]
    pub const fn should_use_alternate_screen(&self) -> bool {
        self.use_alternate_screen
    }

    /// Returns `true` if the cursor should be hidden during the session.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// assert!(TerminalSessionConfig::default().should_hide_cursor());
    /// ```
    #[must_use]
    #[inline]
    pub const fn should_hide_cursor(&self) -> bool {
        self.hide_cursor
    }

    /// Returns `true` if DEC Mode 1007 alternate scroll should be enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::terminal::TerminalSessionConfig;
    ///
    /// assert!(TerminalSessionConfig::default().should_enable_alternate_scroll());
    /// ```
    #[must_use]
    #[inline]
    pub const fn should_enable_alternate_scroll(&self) -> bool {
        self.enable_alternate_scroll
    }
}

/// RAII guard and double-buffered frame renderer for a terminal backend.
///
/// Entering a [`TerminalSessionGuard`] acquires raw mode, enters the alternate
/// screen (if configured), enables DEC Mode 1007 alternate scroll, and hides
/// the cursor. When dropped, all terminal modes are restored in reverse order.
///
/// # Examples
///
/// ```
/// use dashboard_tui::backend::TestBackend;
/// use dashboard_tui::core::{Color, Style, TerminalHeight, TerminalWidth};
/// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
///
/// let backend = TestBackend::new(TerminalWidth::new(40), TerminalHeight::new(10));
/// let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
/// session.draw(|buffer, area| {
///     buffer.set_string(area.x(), area.y(), "Hello VaultDrive", Style::new().with_fg(Color::Green), area.width().get());
/// })?;
/// assert!(session.backend().screen_buffer().to_visual_lines()[0].contains("Hello VaultDrive"));
/// # Ok::<(), dashboard_tui::TuiError>(())
/// ```
#[derive(Debug)]
pub struct TerminalSessionGuard<B: Backend> {
    backend: B,
    config: TerminalSessionConfig,
    current_buffer: Buffer,
    previous_buffer: Buffer,
    needs_full_redraw: bool,
}

impl<B: Backend> TerminalSessionGuard<B> {
    /// Enters a managed terminal session with the provided backend and configuration.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if entering raw mode or switching to the alternate
    /// screen buffer fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::backend::TestBackend;
    /// use dashboard_tui::core::{TerminalHeight, TerminalWidth};
    /// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
    ///
    /// let backend = TestBackend::new(TerminalWidth::new(80), TerminalHeight::new(24));
    /// let session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    /// assert!(session.backend().is_raw_mode_enabled());
    /// # Ok::<(), dashboard_tui::TuiError>(())
    /// ```
    pub fn enter(mut backend: B, config: TerminalSessionConfig) -> Result<Self, TuiError> {
        backend.enable_raw_mode()?;
        if config.use_alternate_screen {
            if let Err(err) = backend.enter_alternate_screen() {
                let _ = backend.disable_raw_mode();
                return Err(err);
            }
        }
        if config.enable_alternate_scroll {
            let _ = backend.enable_alternate_scroll();
        }
        if config.hide_cursor {
            let _ = backend.hide_cursor();
        }
        let (width, height) = backend.dimensions();
        if let Viewport::Inline(rows) = config.viewport {
            // Push existing output up so the inline viewport has fresh rows to draw in.
            let _ = backend.scroll_up(rows.min(height.get()));
        }
        let area = Self::viewport_area(config.viewport, width, height);
        Ok(Self {
            backend,
            config,
            current_buffer: Buffer::empty(area),
            previous_buffer: Buffer::empty(Rect::ZERO),
            needs_full_redraw: true,
        })
    }

    fn viewport_area(viewport: Viewport, width: TerminalWidth, height: TerminalHeight) -> Rect {
        match viewport {
            Viewport::Fullscreen => Rect::from_size(width, height),
            Viewport::Inline(rows) => {
                let shown = rows.clamp(1, height.get().max(1)).min(height.get());
                Rect::new(
                    ColumnIndex::new(0),
                    RowIndex::new(height.get().saturating_sub(shown)),
                    width,
                    TerminalHeight::new(shown),
                )
            }
        }
    }

    /// Returns a shared reference to the underlying backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::backend::TestBackend;
    /// use dashboard_tui::core::{TerminalHeight, TerminalWidth};
    /// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
    ///
    /// let backend = TestBackend::new(TerminalWidth::new(80), TerminalHeight::new(24));
    /// let session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    /// assert!(session.backend().is_alternate_screen_active());
    /// # Ok::<(), dashboard_tui::TuiError>(())
    /// ```
    #[must_use]
    #[inline]
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns a mutable reference to the underlying backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::backend::TestBackend;
    /// use dashboard_tui::core::{TerminalHeight, TerminalWidth};
    /// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
    ///
    /// let backend = TestBackend::new(TerminalWidth::new(80), TerminalHeight::new(24));
    /// let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    /// session.backend_mut().resize(TerminalWidth::new(100), TerminalHeight::new(30));
    /// # Ok::<(), dashboard_tui::TuiError>(())
    /// ```
    #[inline]
    pub const fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Forces a complete screen clear and full cell redraw on the next [`Self::draw`] call.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::backend::TestBackend;
    /// use dashboard_tui::core::{TerminalHeight, TerminalWidth};
    /// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
    ///
    /// let backend = TestBackend::new(TerminalWidth::new(80), TerminalHeight::new(24));
    /// let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    /// session.request_full_redraw();
    /// # Ok::<(), dashboard_tui::TuiError>(())
    /// ```
    #[inline]
    pub const fn request_full_redraw(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Executes a zero-allocation frame render pass, diffs against the previous
    /// frame buffer, and flushes all mutated cells in a single atomic operation.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if clearing on resize, writing cell diffs, or
    /// flushing the backend fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::backend::TestBackend;
    /// use dashboard_tui::core::{Style, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
    ///
    /// let backend = TestBackend::new(TerminalWidth::new(20), TerminalHeight::new(5));
    /// let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    /// session.draw(|buf, rect| {
    ///     buf.set_string(rect.x(), rect.y(), "Frame 1", Style::default(), rect.width().get());
    /// })?;
    /// assert_eq!(session.backend().screen_buffer().to_visual_lines()[0].trim(), "Frame 1");
    /// # Ok::<(), dashboard_tui::TuiError>(())
    /// ```
    pub fn draw<F>(&mut self, render_fn: F) -> Result<Rect, TuiError>
    where
        F: FnOnce(&mut Buffer, Rect),
    {
        self.draw_with_cursor(|buffer, area| {
            render_fn(buffer, area);
            None
        })
    }

    /// Like [`TerminalSessionGuard::draw`], but the closure may return a cursor position.
    ///
    /// `Some((column, row))` shows the hardware cursor there (for text inputs); `None` hides it
    /// again when the session hides the cursor. Coordinates are absolute screen cells.
    ///
    /// # Errors
    ///
    /// Returns [`TuiError`] if drawing, cursor control, or flushing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::backend::TestBackend;
    /// use dashboard_tui::core::{ColumnIndex, RowIndex, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::terminal::{TerminalSessionConfig, TerminalSessionGuard};
    ///
    /// let backend = TestBackend::new(TerminalWidth(20), TerminalHeight(4));
    /// let mut session = TerminalSessionGuard::enter(backend, TerminalSessionConfig::default())?;
    /// session.draw_with_cursor(|_buffer, _area| Some((ColumnIndex(5), RowIndex(2))))?;
    /// assert!(session.backend().is_cursor_visible());
    /// assert_eq!(session.backend().cursor_position(), (ColumnIndex(5), RowIndex(2)));
    /// session.draw_with_cursor(|_buffer, _area| None)?;
    /// assert!(!session.backend().is_cursor_visible());
    /// # Ok::<(), dashboard_tui::error::TuiError>(())
    /// ```
    pub fn draw_with_cursor<F>(&mut self, render_fn: F) -> Result<Rect, TuiError>
    where
        F: FnOnce(&mut Buffer, Rect) -> Option<(ColumnIndex, RowIndex)>,
    {
        let (width, height) = self.backend.dimensions();
        let area = Self::viewport_area(self.config.viewport, width, height);

        if self.current_buffer.area() != area {
            self.current_buffer.resize(area);
            self.needs_full_redraw = true;
        }

        if self.needs_full_redraw {
            // An inline viewport must not wipe the scrollback above it; the forced diff below
            // repaints every cell of the viewport anyway.
            if self.config.viewport == Viewport::Fullscreen {
                self.backend.clear_screen()?;
            }
            // Setting previous_buffer to ZERO bounds causes diff_buffers to set force_all = true
            self.previous_buffer.resize(Rect::ZERO);
            self.needs_full_redraw = false;
        }

        self.current_buffer.reset();
        let cursor = render_fn(&mut self.current_buffer, area);

        {
            let diffs = diff_buffers(&self.previous_buffer, &self.current_buffer);
            self.backend.draw_frame(diffs)?;
        }

        match cursor {
            Some((column, row)) => {
                self.backend.set_cursor_position(column, row)?;
                self.backend.show_cursor()?;
            }
            None if self.config.hide_cursor => self.backend.hide_cursor()?,
            None => {}
        }

        self.backend.flush()?;
        if self.previous_buffer.area() != area {
            self.previous_buffer.resize(area);
        }
        core::mem::swap(&mut self.previous_buffer, &mut self.current_buffer);
        Ok(area)
    }
}

impl<B: Backend> Drop for TerminalSessionGuard<B> {
    fn drop(&mut self) {
        if self.config.enable_alternate_scroll {
            let _ = self.backend.disable_alternate_scroll();
        }
        if self.config.hide_cursor {
            let _ = self.backend.show_cursor();
        }
        if self.config.use_alternate_screen {
            let _ = self.backend.leave_alternate_screen();
        }
        if matches!(self.config.viewport, Viewport::Inline(_)) {
            // Leave the last frame in the scrollback and park the cursor on a fresh line below it.
            let (_, height) = self.backend.dimensions();
            let _ = self.backend.scroll_up(1);
            let _ = self.backend.set_cursor_position(
                ColumnIndex::new(0),
                RowIndex::new(height.get().saturating_sub(1)),
            );
        }
        let _ = self.backend.disable_raw_mode();
        let _ = self.backend.flush();
    }
}
