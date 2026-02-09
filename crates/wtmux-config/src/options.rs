/// Terminal multiplexer options with 3-tier inheritance.
#[derive(Debug, Clone)]
pub struct Options {
    // Status bar
    pub status: bool,
    pub status_left: String,
    pub status_right: String,
    pub status_interval: u64,
    pub status_style_fg: String,
    pub status_style_bg: String,

    // Window
    pub base_index: usize,
    pub renumber_windows: bool,
    pub automatic_rename: bool,

    // Terminal
    pub default_shell: String,
    pub default_terminal: String,
    pub escape_time: u64,
    pub history_limit: usize,

    // Mouse
    pub mouse: bool,

    // Prefix
    pub prefix: String,

    // Display
    pub display_time: u64,
    pub display_panes_time: u64,
    pub pane_border_style: String,
    pub pane_active_border_style: String,
}

impl Default for Options {
    fn default() -> Self {
        let default_shell = std::env::var("COMSPEC")
            .unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".to_string());

        Options {
            status: true,
            status_left: "[#{session_name}] ".to_string(),
            status_right: " %H:%M %Y-%m-%d".to_string(),
            status_interval: 1,
            status_style_fg: "black".to_string(),
            status_style_bg: "green".to_string(),

            base_index: 0,
            renumber_windows: false,
            automatic_rename: true,

            default_shell,
            default_terminal: "xterm-256color".to_string(),
            escape_time: 500,
            history_limit: 2000,

            mouse: false,

            prefix: "C-b".to_string(),

            display_time: 750,
            display_panes_time: 1000,
            pane_border_style: "default".to_string(),
            pane_active_border_style: "fg=green".to_string(),
        }
    }
}

impl Options {
    /// Set an option by name.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "status" => self.status = parse_bool(value)?,
            "status-left" => self.status_left = unquote(value),
            "status-right" => self.status_right = unquote(value),
            "status-interval" => {
                self.status_interval = value.parse().map_err(|e| format!("{}", e))?
            }
            "status-style" => {
                // Parse "fg=color,bg=color"
                for part in value.split(',') {
                    let part = part.trim();
                    if let Some(fg) = part.strip_prefix("fg=") {
                        self.status_style_fg = fg.to_string();
                    } else if let Some(bg) = part.strip_prefix("bg=") {
                        self.status_style_bg = bg.to_string();
                    }
                }
            }
            "base-index" => self.base_index = value.parse().map_err(|e| format!("{}", e))?,
            "renumber-windows" => self.renumber_windows = parse_bool(value)?,
            "automatic-rename" => self.automatic_rename = parse_bool(value)?,
            "default-shell" | "default-command" => self.default_shell = unquote(value),
            "default-terminal" => self.default_terminal = unquote(value),
            "escape-time" => self.escape_time = value.parse().map_err(|e| format!("{}", e))?,
            "history-limit" => self.history_limit = value.parse().map_err(|e| format!("{}", e))?,
            "mouse" => self.mouse = parse_bool(value)?,
            "prefix" => self.prefix = value.to_string(),
            "display-time" => self.display_time = value.parse().map_err(|e| format!("{}", e))?,
            "display-panes-time" => {
                self.display_panes_time = value.parse().map_err(|e| format!("{}", e))?
            }
            "pane-border-style" => self.pane_border_style = unquote(value),
            "pane-active-border-style" => self.pane_active_border_style = unquote(value),
            _ => return Err(format!("Unknown option: {}", name)),
        }
        Ok(())
    }

    /// Get an option value by name (as string).
    pub fn get(&self, name: &str) -> Option<String> {
        match name {
            "status" => Some(if self.status { "on" } else { "off" }.to_string()),
            "status-left" => Some(self.status_left.clone()),
            "status-right" => Some(self.status_right.clone()),
            "status-interval" => Some(self.status_interval.to_string()),
            "base-index" => Some(self.base_index.to_string()),
            "default-shell" => Some(self.default_shell.clone()),
            "default-terminal" => Some(self.default_terminal.clone()),
            "escape-time" => Some(self.escape_time.to_string()),
            "history-limit" => Some(self.history_limit.to_string()),
            "mouse" => Some(if self.mouse { "on" } else { "off" }.to_string()),
            "prefix" => Some(self.prefix.clone()),
            _ => None,
        }
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        _ => Err(format!("Invalid boolean value: {}", s)),
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}
