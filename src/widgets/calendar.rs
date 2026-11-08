//! Month calendar (`Calendar`) with selectable and highlighted days.
//!
//! Date arithmetic is self-contained (proleptic Gregorian calendar), so the widget needs no
//! date-time dependency: you pass a year and a month.

use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::grapheme::display_width;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::widget::Widget;

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// First day of the week shown in the leftmost column.
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::WeekStart;
/// assert_eq!(WeekStart::default(), WeekStart::Monday);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WeekStart {
    /// Weeks start on Monday (ISO 8601).
    #[default]
    Monday,
    /// Weeks start on Sunday.
    Sunday,
}

/// Returns `true` if `year` is a leap year in the Gregorian calendar.
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::is_leap_year;
/// assert!(is_leap_year(2024) && !is_leap_year(2100) && is_leap_year(2000));
/// ```
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Returns the number of days in `month` (1–12; other values are clamped).
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::days_in_month;
/// assert_eq!(days_in_month(2024, 2), 29);
/// assert_eq!(days_in_month(2026, 11), 30);
/// ```
#[must_use]
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Returns the weekday of the first of the month, `0` = Sunday … `6` = Saturday.
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::first_weekday_of_month;
/// assert_eq!(first_weekday_of_month(2026, 11), 0); // 1 November 2026 is a Sunday
/// ```
#[must_use]
pub fn first_weekday_of_month(year: i32, month: u8) -> u8 {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let month = month.clamp(1, 12);
    let adjusted_year = if month < 3 { year - 1 } else { year };
    let value = adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + OFFSETS[usize::from(month - 1)]
        + 1;
    u8::try_from(value.rem_euclid(7)).unwrap_or(0)
}

/// A one-month calendar grid.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Rect, Style, Widget};
/// use tui_rs::widgets::Calendar;
///
/// let area = Rect::from_xywh(0, 0, 20, 8);
/// let mut buffer = Buffer::empty(area);
/// Calendar::new(2026, 11).selected(Some(8)).render(area, &mut buffer);
/// let lines = buffer.to_visual_lines();
/// assert_eq!(lines[0].trim(), "November 2026");
/// assert_eq!(lines[1].trim_end(), "Mo Tu We Th Fr Sa Su");
/// assert_eq!(lines[2].trim_end(), "                   1");
/// assert!(lines[3].contains(" 8"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calendar {
    year: i32,
    month: u8,
    selected: Option<u8>,
    marked: Vec<(u8, Style)>,
    week_start: WeekStart,
    style: Style,
    header_style: Style,
    weekday_style: Style,
    selected_style: Style,
    show_header: bool,
}

impl Calendar {
    /// Creates a calendar for `month` (1–12, clamped) of `year`.
    #[must_use]
    pub fn new(year: i32, month: u8) -> Self {
        Self {
            year,
            month: month.clamp(1, 12),
            selected: None,
            marked: Vec::new(),
            week_start: WeekStart::Monday,
            style: Style::DEFAULT,
            header_style: Style::DEFAULT.bold(),
            weekday_style: Style::DEFAULT.dim(),
            selected_style: Style::DEFAULT.reversed(),
            show_header: true,
        }
    }

    /// Highlights day `day` (ignored if outside the month).
    #[must_use]
    pub const fn selected(mut self, day: Option<u8>) -> Self {
        self.selected = day;
        self
    }

    /// Marks `day` with `style` (events, deadlines, …).
    #[must_use]
    pub fn mark(mut self, day: u8, style: Style) -> Self {
        self.marked.push((day, style));
        self
    }

    /// Chooses the first weekday column.
    #[must_use]
    pub const fn week_start(mut self, week_start: WeekStart) -> Self {
        self.week_start = week_start;
        self
    }

    /// Sets the base style of day numbers.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the style of the month/year header (default bold).
    #[must_use]
    pub const fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Sets the style of the weekday names (default dim).
    #[must_use]
    pub const fn weekday_style(mut self, style: Style) -> Self {
        self.weekday_style = style;
        self
    }

    /// Sets the style of the selected day (default reversed).
    #[must_use]
    pub const fn selected_style(mut self, style: Style) -> Self {
        self.selected_style = style;
        self
    }

    /// Shows or hides the `"Month Year"` header line (default shown).
    #[must_use]
    pub const fn show_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }
}

impl Widget for Calendar {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let right_edge = area.right();
        let mut row = area.y();

        if self.show_header {
            let mut header = StackStr::<32>::new();
            let name = MONTH_NAMES[usize::from(self.month - 1)];
            let _ = write!(header, "{name} {}", self.year);
            let width = display_width(header.as_str()).as_u16().min(20);
            let start = area.x().saturating_add(20_u16.saturating_sub(width) / 2);
            let _ = buffer.set_string(start, row, header.as_str(), self.header_style, width);
            row = row.saturating_add(1);
        }

        let names: [&str; 7] = match self.week_start {
            WeekStart::Monday => ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
            WeekStart::Sunday => ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
        };
        for (index, name) in names.iter().enumerate() {
            let column = area
                .x()
                .saturating_add(u16::try_from(index).unwrap_or(0).saturating_mul(3));
            let _ = buffer.set_str_clipped(column, row, name, right_edge, self.weekday_style);
        }
        row = row.saturating_add(1);

        let first = first_weekday_of_month(self.year, self.month);
        let leading = match self.week_start {
            WeekStart::Monday => (first + 6) % 7,
            WeekStart::Sunday => first,
        };
        let days = days_in_month(self.year, self.month);
        for day in 1..=days {
            let slot = u16::from(leading) + u16::from(day) - 1;
            let week = slot / 7;
            let weekday = slot % 7;
            let y = RowIndex::new(row.get().saturating_add(week));
            if y >= area.bottom() {
                break;
            }
            let x = ColumnIndex::new(area.x().get().saturating_add(weekday * 3));
            let mut style = self.style;
            if let Some((_, mark)) = self.marked.iter().find(|(marked, _)| *marked == day) {
                style = style.patch(*mark);
            }
            if self.selected == Some(day) {
                style = style.patch(self.selected_style);
            }
            let mut text = StackStr::<4>::new();
            let _ = write!(text, "{day:>2}");
            let _ = buffer.set_str_clipped(x, y, text.as_str(), right_edge, style);
        }
    }
}
