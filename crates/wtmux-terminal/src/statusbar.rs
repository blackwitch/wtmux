use crate::cell::{Cell, Color};

/// Status bar configuration and rendering.
pub struct StatusBar {
    pub left_format: String,
    pub right_format: String,
    pub style_fg: Color,
    pub style_bg: Color,
    pub active_window_fg: Color,
    pub active_window_bg: Color,
}

impl Default for StatusBar {
    fn default() -> Self {
        StatusBar {
            left_format: "[#{session_name}] ".to_string(),
            right_format: " %H:%M %Y-%m-%d".to_string(),
            style_fg: Color::Indexed(0),      // black
            style_bg: Color::Indexed(2),       // green
            active_window_fg: Color::Indexed(0),
            active_window_bg: Color::Indexed(3), // yellow
        }
    }
}

/// Information needed to render the status bar.
pub struct StatusBarContext {
    pub session_name: String,
    pub windows: Vec<WindowStatus>,
    pub cols: u16,
}

pub struct WindowStatus {
    pub index: usize,
    pub name: String,
    pub active: bool,
}

impl StatusBar {
    /// Render the status bar as a row of cells.
    pub fn render(&self, ctx: &StatusBarContext) -> Vec<Cell> {
        let cols = ctx.cols as usize;
        let mut cells = vec![
            Cell {
                ch: ' ',
                fg: self.style_fg,
                bg: self.style_bg,
                ..Default::default()
            };
            cols
        ];

        // Render left section: session name + window list
        let left = self.expand_format(&self.left_format, ctx);
        let mut pos = 0;
        for ch in left.chars() {
            if pos >= cols {
                break;
            }
            cells[pos].ch = ch;
            pos += 1;
        }

        // Render window list
        for win in &ctx.windows {
            let label = format!("{}:{}", win.index, win.name);
            let suffix = if win.active { "* " } else { " " };
            let full = format!("{}{}", label, suffix);

            for ch in full.chars() {
                if pos >= cols {
                    break;
                }
                cells[pos].ch = ch;
                if win.active {
                    cells[pos].fg = self.active_window_fg;
                    cells[pos].bg = self.active_window_bg;
                }
                pos += 1;
            }
        }

        // Render right section
        let right = self.expand_format(&self.right_format, ctx);
        let right_start = cols.saturating_sub(right.len());
        let mut pos = right_start;
        for ch in right.chars() {
            if pos >= cols {
                break;
            }
            cells[pos].ch = ch;
            pos += 1;
        }

        cells
    }

    fn expand_format(&self, format: &str, ctx: &StatusBarContext) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();

        // Simple time calculations (UTC)
        let hours = (secs % 86400) / 3600;
        let minutes = (secs % 3600) / 60;
        let days = secs / 86400;
        // Approximate date calculation
        let (year, month, day) = days_to_ymd(days);

        format
            .replace("#{session_name}", &ctx.session_name)
            .replace("%H", &format!("{:02}", hours))
            .replace("%M", &format!("{:02}", minutes))
            .replace("%Y", &format!("{:04}", year))
            .replace("%m", &format!("{:02}", month))
            .replace("%d", &format!("{:02}", day))
    }
}

/// Convert days since epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Simplified calculation
    let mut y = 1970;
    let mut remaining = days as i64;

    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let months = if is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 1;
    for &days_in_month in &months {
        if remaining < days_in_month {
            break;
        }
        remaining -= days_in_month;
        m += 1;
    }

    (y as u64, m as u64, remaining as u64 + 1)
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
