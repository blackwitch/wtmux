use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use wtmux_common::{PaneId, SessionId, WindowId};
use wtmux_layout::geometry::Rect;

use crate::pane::Pane;
use crate::window::Window;

/// A session contains one or more windows.
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<Window>,
    pub active_window_idx: usize,
    pub last_window_idx: Option<usize>,
    pub created_at: u64,
    next_window_index: usize,
}

impl Session {
    pub fn new(name: String, command: &str, cols: u16, rows: u16) -> Result<Self> {
        let id = SessionId::new();
        let area = Rect::new(0, 0, cols, rows.saturating_sub(1)); // Reserve 1 row for status bar
        let pane = Pane::new(command, area.width, area.height)?;

        let window = Window::new("cmd".to_string(), 0, pane, area);

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Session {
            id,
            name,
            windows: vec![window],
            active_window_idx: 0,
            last_window_idx: None,
            created_at,
            next_window_index: 1,
        })
    }

    /// Get the active window.
    pub fn active_window(&self) -> &Window {
        &self.windows[self.active_window_idx]
    }

    /// Get the active window mutably.
    pub fn active_window_mut(&mut self) -> &mut Window {
        &mut self.windows[self.active_window_idx]
    }

    /// Get the active pane ID.
    pub fn active_pane_id(&self) -> PaneId {
        self.active_window().active_pane
    }

    /// Create a new window.
    pub fn new_window(&mut self, name: Option<String>, command: &str, cols: u16, rows: u16) -> Result<WindowId> {
        let area = Rect::new(0, 0, cols, rows.saturating_sub(1));
        let pane = Pane::new(command, area.width, area.height)?;
        let idx = self.next_window_index;
        self.next_window_index += 1;

        let win_name = name.unwrap_or_else(|| "cmd".to_string());
        let window = Window::new(win_name, idx, pane, area);
        let win_id = window.id;

        self.windows.push(window);
        self.last_window_idx = Some(self.active_window_idx);
        self.active_window_idx = self.windows.len() - 1;

        Ok(win_id)
    }

    /// Select a window by index number.
    pub fn select_window(&mut self, index: usize) -> bool {
        if let Some(pos) = self.windows.iter().position(|w| w.index == index) {
            if pos != self.active_window_idx {
                self.last_window_idx = Some(self.active_window_idx);
            }
            self.active_window_idx = pos;
            true
        } else {
            false
        }
    }

    /// Next window.
    pub fn next_window(&mut self) {
        if !self.windows.is_empty() {
            self.last_window_idx = Some(self.active_window_idx);
            self.active_window_idx = (self.active_window_idx + 1) % self.windows.len();
        }
    }

    /// Previous window.
    pub fn prev_window(&mut self) {
        if !self.windows.is_empty() {
            self.last_window_idx = Some(self.active_window_idx);
            self.active_window_idx = if self.active_window_idx == 0 {
                self.windows.len() - 1
            } else {
                self.active_window_idx - 1
            };
        }
    }

    /// Select the last active window (Ctrl-B l).
    pub fn select_last_window(&mut self) -> bool {
        if let Some(last) = self.last_window_idx {
            if last < self.windows.len() {
                let old = self.active_window_idx;
                self.active_window_idx = last;
                self.last_window_idx = Some(old);
                return true;
            }
        }
        false
    }

    /// Close a window. Returns true if the session is now empty.
    pub fn close_window(&mut self, window_id: WindowId) -> bool {
        if let Some(pos) = self.windows.iter().position(|w| w.id == window_id) {
            self.windows.remove(pos);
            if self.active_window_idx >= self.windows.len() && !self.windows.is_empty() {
                self.active_window_idx = self.windows.len() - 1;
            }
        }
        self.windows.is_empty()
    }

    /// Resize all windows in the session.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let area = Rect::new(0, 0, cols, rows.saturating_sub(1));
        for window in &mut self.windows {
            window.resize(area)?;
        }
        Ok(())
    }

    /// Get the total pane count across all windows.
    pub fn pane_count(&self) -> usize {
        self.windows.iter().map(|w| w.pane_count()).sum()
    }

    /// Get window info list for status bar.
    pub fn window_infos(&self) -> Vec<wtmux_common::protocol::WindowInfo> {
        self.windows
            .iter()
            .enumerate()
            .map(|(i, w)| wtmux_common::protocol::WindowInfo {
                id: w.id,
                index: w.index,
                name: w.name.clone(),
                active: i == self.active_window_idx,
                pane_count: w.pane_count(),
            })
            .collect()
    }
}
