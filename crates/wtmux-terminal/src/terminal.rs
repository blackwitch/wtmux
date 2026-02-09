use crate::cell::{Attrs, Color};
use crate::parser::TerminalState;

/// High-level terminal that wraps VT parsing and grid management.
pub struct Terminal {
    pub state: TerminalState,
    vt_parser: vte::Parser,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Terminal {
            state: TerminalState::new(cols, rows),
            vt_parser: vte::Parser::new(),
        }
    }

    /// Process raw bytes from the PTY, updating the internal grid.
    pub fn process_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.vt_parser.advance(&mut self.state, byte);
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.state.resize(cols, rows);
    }

    /// Get the current cursor position.
    pub fn cursor_pos(&self) -> (u16, u16) {
        (self.state.cursor.col, self.state.cursor.row)
    }

    /// Check if the terminal content has been modified.
    pub fn is_dirty(&self) -> bool {
        self.state.dirty
    }

    /// Mark the terminal as clean (after rendering).
    pub fn mark_clean(&mut self) {
        self.state.dirty = false;
    }

    /// Render the terminal grid to ANSI escape sequences.
    pub fn render(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(
            (self.state.grid.cols as usize * self.state.grid.rows as usize) * 4,
        );

        // Hide cursor during render
        output.extend_from_slice(b"\x1b[?25l");
        // Move to home position
        output.extend_from_slice(b"\x1b[H");

        let mut prev_fg = Color::Default;
        let mut prev_bg = Color::Default;
        let mut prev_attrs = Attrs::default();

        for row in 0..self.state.grid.rows {
            if row > 0 {
                output.extend_from_slice(b"\r\n");
            }

            for col in 0..self.state.grid.cols {
                let cell = self.state.grid.cell(col, row);

                // Skip continuation cells for wide characters
                if cell.width == 0 {
                    continue;
                }

                // Emit SGR changes only when needed
                let need_sgr = cell.fg != prev_fg
                    || cell.bg != prev_bg
                    || cell.attrs != prev_attrs;

                if need_sgr {
                    output.extend_from_slice(b"\x1b[0"); // Reset first

                    // Foreground
                    write_color(&mut output, cell.fg, true);
                    // Background
                    write_color(&mut output, cell.bg, false);
                    // Attributes
                    write_attrs(&mut output, cell.attrs);

                    output.push(b'm');

                    prev_fg = cell.fg;
                    prev_bg = cell.bg;
                    prev_attrs = cell.attrs;
                }

                // Write the character
                let mut buf = [0u8; 4];
                let s = cell.ch.encode_utf8(&mut buf);
                output.extend_from_slice(s.as_bytes());
            }
        }

        // Reset attributes
        output.extend_from_slice(b"\x1b[0m");

        // Restore cursor position and visibility
        output.extend_from_slice(
            format!(
                "\x1b[{};{}H",
                self.state.cursor.row + 1,
                self.state.cursor.col + 1
            )
            .as_bytes(),
        );

        if self.state.cursor.visible {
            output.extend_from_slice(b"\x1b[?25h");
        }

        output
    }

    /// Render a rectangular sub-region of the grid to ANSI escape sequences.
    pub fn render_region(&self, x: u16, y: u16, width: u16, height: u16, dest_x: u16, dest_y: u16) -> Vec<u8> {
        let mut output = Vec::new();

        let mut prev_fg = Color::Default;
        let mut prev_bg = Color::Default;
        let mut prev_attrs = Attrs::default();

        for row_offset in 0..height {
            let src_row = y + row_offset;
            let dst_row = dest_y + row_offset;

            if src_row >= self.state.grid.rows {
                break;
            }

            // Move cursor to destination position
            output.extend_from_slice(
                format!("\x1b[{};{}H", dst_row + 1, dest_x + 1).as_bytes(),
            );

            for col_offset in 0..width {
                let src_col = x + col_offset;
                if src_col >= self.state.grid.cols {
                    break;
                }

                let cell = self.state.grid.cell(src_col, src_row);
                if cell.width == 0 {
                    continue;
                }

                let need_sgr = cell.fg != prev_fg
                    || cell.bg != prev_bg
                    || cell.attrs != prev_attrs;

                if need_sgr {
                    output.extend_from_slice(b"\x1b[0");
                    write_color(&mut output, cell.fg, true);
                    write_color(&mut output, cell.bg, false);
                    write_attrs(&mut output, cell.attrs);
                    output.push(b'm');
                    prev_fg = cell.fg;
                    prev_bg = cell.bg;
                    prev_attrs = cell.attrs;
                }

                let mut buf = [0u8; 4];
                let s = cell.ch.encode_utf8(&mut buf);
                output.extend_from_slice(s.as_bytes());
            }
        }

        output.extend_from_slice(b"\x1b[0m");
        output
    }
}

fn write_color(output: &mut Vec<u8>, color: Color, is_fg: bool) {
    match color {
        Color::Default => {}
        Color::Indexed(n) if n < 8 => {
            let base = if is_fg { 30 } else { 40 };
            output.extend_from_slice(format!(";{}", base + n).as_bytes());
        }
        Color::Indexed(n) if n < 16 => {
            let base = if is_fg { 90 } else { 100 };
            output.extend_from_slice(format!(";{}", base + n - 8).as_bytes());
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

fn write_attrs(output: &mut Vec<u8>, attrs: Attrs) {
    if attrs.bold {
        output.extend_from_slice(b";1");
    }
    if attrs.italic {
        output.extend_from_slice(b";3");
    }
    if attrs.underline {
        output.extend_from_slice(b";4");
    }
    if attrs.blink {
        output.extend_from_slice(b";5");
    }
    if attrs.reverse {
        output.extend_from_slice(b";7");
    }
    if attrs.hidden {
        output.extend_from_slice(b";8");
    }
    if attrs.strikethrough {
        output.extend_from_slice(b";9");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_simple_text() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"Hello");
        assert_eq!(term.state.grid.cell(0, 0).ch, 'H');
        assert_eq!(term.state.grid.cell(1, 0).ch, 'e');
        assert_eq!(term.state.grid.cell(4, 0).ch, 'o');
        assert_eq!(term.cursor_pos(), (5, 0));
    }

    #[test]
    fn test_process_newline() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"Hello\r\nWorld");
        assert_eq!(term.state.grid.cell(0, 0).ch, 'H');
        assert_eq!(term.state.grid.cell(0, 1).ch, 'W');
    }

    #[test]
    fn test_cursor_movement() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"\x1b[5;10H");
        assert_eq!(term.cursor_pos(), (9, 4));
    }

    #[test]
    fn test_clear_screen() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"Hello\x1b[2J");
        assert_eq!(term.state.grid.cell(0, 0).ch, ' ');
    }

    #[test]
    fn test_sgr_colors() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"\x1b[31mRed");
        assert_eq!(term.state.grid.cell(0, 0).fg, Color::Indexed(1));
    }
}
