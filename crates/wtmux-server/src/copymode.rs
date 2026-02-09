use wtmux_common::protocol::CopyModeAction;
use wtmux_terminal::Terminal;

/// Copy mode state for a pane.
pub struct CopyMode {
    pub active: bool,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub scroll_offset: usize,
    pub selection_start: Option<(u16, u16)>,
    pub selection_end: Option<(u16, u16)>,
    pub search_query: String,
    pub search_direction_forward: bool,
}

impl CopyMode {
    pub fn new(cursor_x: u16, cursor_y: u16) -> Self {
        CopyMode {
            active: true,
            cursor_x,
            cursor_y,
            scroll_offset: 0,
            selection_start: None,
            selection_end: None,
            search_query: String::new(),
            search_direction_forward: true,
        }
    }

    /// Handle a copy mode action. Returns Some(text) if text was copied.
    pub fn handle_action(
        &mut self,
        action: &CopyModeAction,
        terminal: &Terminal,
    ) -> Option<String> {
        let cols = terminal.state.grid.cols;
        let rows = terminal.state.grid.rows;

        match action {
            CopyModeAction::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                } else {
                    self.scroll_offset += 1;
                }
            }
            CopyModeAction::Down => {
                if self.cursor_y < rows - 1 {
                    self.cursor_y += 1;
                } else if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            CopyModeAction::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            }
            CopyModeAction::Right => {
                if self.cursor_x < cols - 1 {
                    self.cursor_x += 1;
                }
            }
            CopyModeAction::PageUp => {
                self.scroll_offset += rows as usize;
            }
            CopyModeAction::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(rows as usize);
            }
            CopyModeAction::HalfPageUp => {
                self.scroll_offset += (rows / 2) as usize;
            }
            CopyModeAction::HalfPageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub((rows / 2) as usize);
            }
            CopyModeAction::Top => {
                self.cursor_y = 0;
                // scroll_offset to maximum
            }
            CopyModeAction::Bottom => {
                self.cursor_y = rows - 1;
                self.scroll_offset = 0;
            }
            CopyModeAction::StartOfLine => {
                self.cursor_x = 0;
            }
            CopyModeAction::EndOfLine => {
                self.cursor_x = cols - 1;
            }
            CopyModeAction::StartSelection => {
                self.selection_start = Some((self.cursor_x, self.cursor_y));
                self.selection_end = None;
            }
            CopyModeAction::CopySelection => {
                if let Some(start) = self.selection_start {
                    self.selection_end = Some((self.cursor_x, self.cursor_y));
                    let text = self.extract_selection(terminal, start, (self.cursor_x, self.cursor_y));
                    self.active = false;
                    return Some(text);
                }
            }
            CopyModeAction::CancelSelection => {
                self.selection_start = None;
                self.selection_end = None;
            }
            CopyModeAction::SearchForward(query) => {
                self.search_query = query.clone();
                self.search_direction_forward = true;
                self.do_search(terminal);
            }
            CopyModeAction::SearchBackward(query) => {
                self.search_query = query.clone();
                self.search_direction_forward = false;
                self.do_search(terminal);
            }
            CopyModeAction::SearchNext => {
                self.do_search(terminal);
            }
            CopyModeAction::SearchPrev => {
                self.search_direction_forward = !self.search_direction_forward;
                self.do_search(terminal);
                self.search_direction_forward = !self.search_direction_forward;
            }
            CopyModeAction::Exit => {
                self.active = false;
            }
        }

        None
    }

    fn extract_selection(
        &self,
        terminal: &Terminal,
        start: (u16, u16),
        end: (u16, u16),
    ) -> String {
        let mut text = String::new();
        let cols = terminal.state.grid.cols;

        let (start_row, start_col, end_row, end_col) = if start.1 < end.1
            || (start.1 == end.1 && start.0 <= end.0)
        {
            (start.1, start.0, end.1, end.0)
        } else {
            (end.1, end.0, start.1, start.0)
        };

        for row in start_row..=end_row {
            if row >= terminal.state.grid.rows {
                break;
            }
            let col_start = if row == start_row { start_col } else { 0 };
            let col_end = if row == end_row { end_col } else { cols - 1 };

            for col in col_start..=col_end {
                if col >= cols {
                    break;
                }
                let cell = terminal.state.grid.cell(col, row);
                if cell.width > 0 {
                    text.push(cell.ch);
                }
            }

            if row != end_row {
                // Trim trailing spaces and add newline
                let trimmed = text.trim_end().len();
                text.truncate(trimmed);
                text.push('\n');
            }
        }

        text
    }

    fn do_search(&mut self, terminal: &Terminal) {
        if self.search_query.is_empty() {
            return;
        }

        if let Some((col, row)) = terminal.state.grid.search(
            &self.search_query,
            self.cursor_x,
            self.cursor_y,
            self.search_direction_forward,
        ) {
            self.cursor_x = col;
            self.cursor_y = row;
        }
    }

    /// Render copy mode indicator.
    pub fn render_indicator(&self) -> Vec<u8> {
        let mut output = Vec::new();
        // Show copy mode indicator in top-right
        let indicator = if self.selection_start.is_some() {
            "[Copy mode - selecting]"
        } else {
            "[Copy mode]"
        };

        output.extend_from_slice(
            format!("\x1b[1;1H\x1b[43;30m{}\x1b[0m", indicator).as_bytes(),
        );
        output
    }
}
