use std::collections::HashMap;
use wtmux_common::PaneId;
use wtmux_layout::geometry::Rect;
use wtmux_terminal::cell::Color;
use wtmux_terminal::statusbar::{StatusBar, StatusBarContext, WindowStatus};

use crate::session::Session;

/// Compose pane grids, borders, and status bar into a final screen buffer.
pub struct Renderer {
    pub cols: u16,
    pub rows: u16,
    status_bar: StatusBar,
}

impl Renderer {
    pub fn new(cols: u16, rows: u16) -> Self {
        Renderer {
            cols,
            rows,
            status_bar: StatusBar::default(),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }

    /// Render the entire screen for a session.
    pub fn render(&self, session: &Session) -> Vec<u8> {
        let mut output = Vec::with_capacity((self.cols as usize * self.rows as usize) * 4);

        // Hide cursor during render
        output.extend_from_slice(b"\x1b[?25l");

        let window = session.active_window();
        let geometries = window.pane_geometries();

        // Render each pane
        for (pane_id, rect) in &geometries {
            if let Some(pane) = window.panes.get(pane_id) {
                let pane_output = pane.terminal.render_region(
                    0,
                    0,
                    rect.width,
                    rect.height,
                    rect.x,
                    rect.y,
                );
                output.extend_from_slice(&pane_output);
            }
        }

        // Render pane borders if more than one pane and not zoomed
        if window.panes.len() > 1 && window.zoomed_pane.is_none() {
            let border_output =
                self.render_borders(&geometries, window.active_pane);
            output.extend_from_slice(&border_output);
        }

        // Render status bar at the bottom
        let status_output = self.render_status_bar(session);
        output.extend_from_slice(&status_output);

        // Restore cursor to active pane position
        if let Some(pane) = window.panes.get(&window.active_pane) {
            if let Some(rect) = geometries.get(&window.active_pane) {
                let (cx, cy) = pane.terminal.cursor_pos();
                output.extend_from_slice(
                    format!(
                        "\x1b[{};{}H",
                        rect.y + cy + 1,
                        rect.x + cx + 1
                    )
                    .as_bytes(),
                );
            }
        }

        // Show cursor
        output.extend_from_slice(b"\x1b[?25h");

        output
    }

    fn render_borders(
        &self,
        geometries: &HashMap<PaneId, Rect>,
        active_pane: PaneId,
    ) -> Vec<u8> {
        let mut output = Vec::new();

        // Draw borders between panes using box-drawing characters
        for (&pane_id, rect) in geometries {
            let is_active = pane_id == active_pane;
            let color = if is_active {
                "\x1b[32m" // Green for active
            } else {
                "\x1b[37m" // White for inactive
            };

            // Right border (if there's space)
            if rect.right() < self.cols {
                output.extend_from_slice(color.as_bytes());
                for row in rect.y..rect.bottom() {
                    output.extend_from_slice(
                        format!("\x1b[{};{}H│", row + 1, rect.right() + 1).as_bytes(),
                    );
                }
            }

            // Bottom border (if there's space and not at the status bar line)
            if rect.bottom() < self.rows.saturating_sub(1) {
                output.extend_from_slice(color.as_bytes());
                for col in rect.x..rect.right() {
                    output.extend_from_slice(
                        format!("\x1b[{};{}H─", rect.bottom() + 1, col + 1).as_bytes(),
                    );
                }
            }
        }

        output.extend_from_slice(b"\x1b[0m");
        output
    }

    fn render_status_bar(&self, session: &Session) -> Vec<u8> {
        let mut output = Vec::new();

        let ctx = StatusBarContext {
            session_name: session.name.clone(),
            windows: session
                .window_infos()
                .iter()
                .map(|w| WindowStatus {
                    index: w.index,
                    name: w.name.clone(),
                    active: w.active,
                })
                .collect(),
            cols: self.cols,
        };

        let cells = self.status_bar.render(&ctx);

        // Move to status bar position (last row)
        output.extend_from_slice(
            format!("\x1b[{};1H", self.rows).as_bytes(),
        );

        // Render cells
        let mut prev_fg = Color::Default;
        let mut prev_bg = Color::Default;

        for cell in &cells {
            let need_sgr = cell.fg != prev_fg || cell.bg != prev_bg;
            if need_sgr {
                output.extend_from_slice(b"\x1b[0");
                write_status_color(&mut output, cell.fg, true);
                write_status_color(&mut output, cell.bg, false);
                output.push(b'm');
                prev_fg = cell.fg;
                prev_bg = cell.bg;
            }

            let mut buf = [0u8; 4];
            let s = cell.ch.encode_utf8(&mut buf);
            output.extend_from_slice(s.as_bytes());
        }

        output.extend_from_slice(b"\x1b[0m");
        output
    }
}

fn write_status_color(output: &mut Vec<u8>, color: Color, is_fg: bool) {
    match color {
        Color::Default => {}
        Color::Indexed(n) if n < 8 => {
            let base = if is_fg { 30 } else { 40 };
            output.extend_from_slice(format!(";{}", base + n as u32).as_bytes());
        }
        Color::Indexed(n) if n < 16 => {
            let base = if is_fg { 90 } else { 100 };
            output.extend_from_slice(format!(";{}", base + n as u32 - 8).as_bytes());
        }
        Color::Indexed(n) => {
            let prefix = if is_fg { "38" } else { "48" };
            output.extend_from_slice(format!(";{};5;{}", prefix, n).as_bytes());
        }
        Color::Rgb(r, g, b) => {
            let prefix = if is_fg { "38" } else { "48" };
            output.extend_from_slice(format!(";{};2;{};{};{}", prefix, r, g, b).as_bytes());
        }
    }
}
