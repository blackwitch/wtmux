use crate::cell::{Attrs, Cell, Color};
use crate::grid::Grid;
use tracing::trace;
use unicode_width::UnicodeWidthChar;

/// Cursor position and attributes for the terminal.
pub struct Cursor {
    pub col: u16,
    pub row: u16,
    pub attrs: Attrs,
    pub fg: Color,
    pub bg: Color,
    pub visible: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            col: 0,
            row: 0,
            attrs: Attrs::default(),
            fg: Color::Default,
            bg: Color::Default,
            visible: true,
        }
    }
}

/// Terminal state that implements vte::Perform to process VT sequences.
pub struct TerminalState {
    pub grid: Grid,
    pub cursor: Cursor,
    pub scroll_top: u16,
    pub scroll_bottom: u16,
    pub saved_cursor: Option<(u16, u16, Attrs, Color, Color)>,
    pub title: String,
    /// Whether the terminal content has changed since last render.
    pub dirty: bool,
    // Alternate screen buffer support
    alt_grid: Option<Grid>,
    alt_cursor: Option<Cursor>,
    pub using_alt_screen: bool,
}

impl TerminalState {
    pub fn new(cols: u16, rows: u16) -> Self {
        TerminalState {
            grid: Grid::new(cols, rows),
            cursor: Cursor::default(),
            scroll_top: 0,
            scroll_bottom: rows,
            saved_cursor: None,
            title: String::new(),
            dirty: true,
            alt_grid: None,
            alt_cursor: None,
            using_alt_screen: false,
        }
    }

    pub fn cols(&self) -> u16 {
        self.grid.cols
    }

    pub fn rows(&self) -> u16 {
        self.grid.rows
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.grid.resize(cols, rows);
        self.scroll_top = 0;
        self.scroll_bottom = rows;
        if self.cursor.col >= cols {
            self.cursor.col = cols - 1;
        }
        if self.cursor.row >= rows {
            self.cursor.row = rows - 1;
        }
        if let Some(ref mut alt) = self.alt_grid {
            alt.resize(cols, rows);
        }
        self.dirty = true;
    }

    fn advance_cursor(&mut self) {
        self.cursor.col += 1;
        if self.cursor.col >= self.grid.cols {
            self.cursor.col = 0;
            self.line_feed();
        }
    }

    fn line_feed(&mut self) {
        if self.cursor.row + 1 >= self.scroll_bottom {
            self.grid.scroll_up(self.scroll_top, self.scroll_bottom);
        } else {
            self.cursor.row += 1;
        }
    }

    fn enter_alt_screen(&mut self) {
        if !self.using_alt_screen {
            let cols = self.grid.cols;
            let rows = self.grid.rows;
            self.alt_grid = Some(std::mem::replace(&mut self.grid, Grid::new(cols, rows)));
            self.alt_cursor = Some(std::mem::replace(&mut self.cursor, Cursor::default()));
            self.using_alt_screen = true;
        }
    }

    fn exit_alt_screen(&mut self) {
        if self.using_alt_screen {
            if let Some(grid) = self.alt_grid.take() {
                self.grid = grid;
            }
            if let Some(cursor) = self.alt_cursor.take() {
                self.cursor = cursor;
            }
            self.using_alt_screen = false;
        }
    }

    fn parse_color_from_params(&self, params: &[u16], idx: &mut usize) -> Option<Color> {
        if *idx >= params.len() {
            return None;
        }
        match params[*idx] {
            2 => {
                // RGB: 2;r;g;b
                *idx += 1;
                if *idx + 2 < params.len() {
                    let r = params[*idx] as u8;
                    let g = params[*idx + 1] as u8;
                    let b = params[*idx + 2] as u8;
                    *idx += 3;
                    Some(Color::Rgb(r, g, b))
                } else {
                    None
                }
            }
            5 => {
                // Indexed: 5;n
                *idx += 1;
                if *idx < params.len() {
                    let n = params[*idx] as u8;
                    *idx += 1;
                    Some(Color::Indexed(n))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl vte::Perform for TerminalState {
    fn print(&mut self, c: char) {
        let width = c.width().unwrap_or(1) as u8;
        let cell = Cell {
            ch: c,
            fg: self.cursor.fg,
            bg: self.cursor.bg,
            attrs: self.cursor.attrs,
            width,
        };

        if self.cursor.col < self.grid.cols && self.cursor.row < self.grid.rows {
            self.grid
                .set_cell(self.cursor.col, self.cursor.row, cell);
            // For wide characters, mark the next cell as a continuation.
            if width == 2 && self.cursor.col + 1 < self.grid.cols {
                let cont = Cell {
                    ch: ' ',
                    fg: self.cursor.fg,
                    bg: self.cursor.bg,
                    attrs: self.cursor.attrs,
                    width: 0, // continuation cell
                };
                self.grid
                    .set_cell(self.cursor.col + 1, self.cursor.row, cont);
                self.cursor.col += 1;
            }
        }

        self.advance_cursor();
        self.dirty = true;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // BEL
            0x07 => {}
            // BS (backspace)
            0x08 => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            // HT (tab)
            0x09 => {
                let next_tab = ((self.cursor.col / 8) + 1) * 8;
                self.cursor.col = next_tab.min(self.grid.cols - 1);
            }
            // LF, VT, FF
            0x0A | 0x0B | 0x0C => {
                self.line_feed();
                self.dirty = true;
            }
            // CR
            0x0D => {
                self.cursor.col = 0;
            }
            _ => {
                trace!("Unhandled execute byte: 0x{:02x}", byte);
            }
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.len() >= 2 {
            match params[0] {
                // Set window title
                b"0" | b"2" => {
                    if let Ok(title) = std::str::from_utf8(params[1]) {
                        self.title = title.to_string();
                    }
                }
                _ => {}
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
        let p = |idx: usize, default: u16| -> u16 {
            params.get(idx).copied().filter(|&v| v != 0).unwrap_or(default)
        };

        match action {
            // CUU - Cursor Up
            'A' => {
                let n = p(0, 1);
                self.cursor.row = self.cursor.row.saturating_sub(n);
                self.dirty = true;
            }
            // CUB - Cursor Down
            'B' => {
                let n = p(0, 1);
                self.cursor.row = (self.cursor.row + n).min(self.grid.rows - 1);
                self.dirty = true;
            }
            // CUF - Cursor Forward
            'C' => {
                let n = p(0, 1);
                self.cursor.col = (self.cursor.col + n).min(self.grid.cols - 1);
                self.dirty = true;
            }
            // CUB - Cursor Backward
            'D' => {
                let n = p(0, 1);
                self.cursor.col = self.cursor.col.saturating_sub(n);
                self.dirty = true;
            }
            // CNL - Cursor Next Line
            'E' => {
                let n = p(0, 1);
                self.cursor.row = (self.cursor.row + n).min(self.grid.rows - 1);
                self.cursor.col = 0;
                self.dirty = true;
            }
            // CPL - Cursor Previous Line
            'F' => {
                let n = p(0, 1);
                self.cursor.row = self.cursor.row.saturating_sub(n);
                self.cursor.col = 0;
                self.dirty = true;
            }
            // CHA - Cursor Horizontal Absolute
            'G' => {
                let col = p(0, 1).saturating_sub(1);
                self.cursor.col = col.min(self.grid.cols - 1);
                self.dirty = true;
            }
            // CUP - Cursor Position
            'H' | 'f' => {
                let row = p(0, 1).saturating_sub(1);
                let col = p(1, 1).saturating_sub(1);
                self.cursor.row = row.min(self.grid.rows - 1);
                self.cursor.col = col.min(self.grid.cols - 1);
                self.dirty = true;
            }
            // ED - Erase in Display
            'J' => {
                match p(0, 0) {
                    0 => {
                        // Clear from cursor to end
                        self.grid.erase_to_eol(self.cursor.row, self.cursor.col);
                        for row in (self.cursor.row + 1)..self.grid.rows {
                            self.grid.clear_row(row);
                        }
                    }
                    1 => {
                        // Clear from start to cursor
                        self.grid.erase_to_bol(self.cursor.row, self.cursor.col);
                        for row in 0..self.cursor.row {
                            self.grid.clear_row(row);
                        }
                    }
                    2 | 3 => {
                        // Clear entire screen
                        self.grid.clear();
                    }
                    _ => {}
                }
                self.dirty = true;
            }
            // EL - Erase in Line
            'K' => {
                match p(0, 0) {
                    0 => self.grid.erase_to_eol(self.cursor.row, self.cursor.col),
                    1 => self.grid.erase_to_bol(self.cursor.row, self.cursor.col),
                    2 => self.grid.clear_row(self.cursor.row),
                    _ => {}
                }
                self.dirty = true;
            }
            // IL - Insert Lines
            'L' => {
                let n = p(0, 1);
                self.grid
                    .insert_lines(self.cursor.row, n, self.scroll_bottom);
                self.dirty = true;
            }
            // DL - Delete Lines
            'M' => {
                let n = p(0, 1);
                self.grid
                    .delete_lines(self.cursor.row, n, self.scroll_bottom);
                self.dirty = true;
            }
            // DCH - Delete Characters
            'P' => {
                let n = p(0, 1) as usize;
                let row = self.cursor.row;
                let col = self.cursor.col as usize;
                let cols = self.grid.cols as usize;
                let row_cells = self.grid.row_mut(row);
                for i in col..(cols - n).max(col) {
                    row_cells[i] = row_cells[i + n].clone();
                }
                for i in (cols - n)..cols {
                    row_cells[i] = Cell::default();
                }
                self.dirty = true;
            }
            // SU - Scroll Up
            'S' => {
                let n = p(0, 1);
                for _ in 0..n {
                    self.grid.scroll_up(self.scroll_top, self.scroll_bottom);
                }
                self.dirty = true;
            }
            // SD - Scroll Down
            'T' => {
                let n = p(0, 1);
                for _ in 0..n {
                    self.grid.scroll_down(self.scroll_top, self.scroll_bottom);
                }
                self.dirty = true;
            }
            // ICH - Insert Characters
            '@' => {
                let n = p(0, 1) as usize;
                let row = self.cursor.row;
                let col = self.cursor.col as usize;
                let cols = self.grid.cols as usize;
                let row_cells = self.grid.row_mut(row);
                for i in (col + n..cols).rev() {
                    row_cells[i] = row_cells[i - n].clone();
                }
                for i in col..((col + n).min(cols)) {
                    row_cells[i] = Cell::default();
                }
                self.dirty = true;
            }
            // ECH - Erase Characters
            'X' => {
                let n = p(0, 1);
                for i in 0..n {
                    let col = self.cursor.col + i;
                    if col < self.grid.cols {
                        self.grid.set_cell(col, self.cursor.row, Cell::default());
                    }
                }
                self.dirty = true;
            }
            // SGR - Select Graphic Rendition
            'm' => {
                if params.is_empty() {
                    self.cursor.attrs = Attrs::default();
                    self.cursor.fg = Color::Default;
                    self.cursor.bg = Color::Default;
                } else {
                    let mut i = 0;
                    while i < params.len() {
                        match params[i] {
                            0 => {
                                self.cursor.attrs = Attrs::default();
                                self.cursor.fg = Color::Default;
                                self.cursor.bg = Color::Default;
                            }
                            1 => self.cursor.attrs.bold = true,
                            3 => self.cursor.attrs.italic = true,
                            4 => self.cursor.attrs.underline = true,
                            5 => self.cursor.attrs.blink = true,
                            7 => self.cursor.attrs.reverse = true,
                            8 => self.cursor.attrs.hidden = true,
                            9 => self.cursor.attrs.strikethrough = true,
                            22 => self.cursor.attrs.bold = false,
                            23 => self.cursor.attrs.italic = false,
                            24 => self.cursor.attrs.underline = false,
                            25 => self.cursor.attrs.blink = false,
                            27 => self.cursor.attrs.reverse = false,
                            28 => self.cursor.attrs.hidden = false,
                            29 => self.cursor.attrs.strikethrough = false,
                            30..=37 => {
                                self.cursor.fg = Color::Indexed(params[i] as u8 - 30);
                            }
                            38 => {
                                i += 1;
                                if let Some(color) = self.parse_color_from_params(&params, &mut i) {
                                    self.cursor.fg = color;
                                }
                                continue;
                            }
                            39 => self.cursor.fg = Color::Default,
                            40..=47 => {
                                self.cursor.bg = Color::Indexed(params[i] as u8 - 40);
                            }
                            48 => {
                                i += 1;
                                if let Some(color) = self.parse_color_from_params(&params, &mut i) {
                                    self.cursor.bg = color;
                                }
                                continue;
                            }
                            49 => self.cursor.bg = Color::Default,
                            90..=97 => {
                                self.cursor.fg = Color::Indexed(params[i] as u8 - 90 + 8);
                            }
                            100..=107 => {
                                self.cursor.bg = Color::Indexed(params[i] as u8 - 100 + 8);
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
                self.dirty = true;
            }
            // DECSTBM - Set Scrolling Region
            'r' => {
                let top = p(0, 1).saturating_sub(1);
                let bottom = p(1, self.grid.rows);
                self.scroll_top = top;
                self.scroll_bottom = bottom;
                self.cursor.col = 0;
                self.cursor.row = 0;
                self.dirty = true;
            }
            // DECSC - Save Cursor
            's' => {
                self.saved_cursor = Some((
                    self.cursor.col,
                    self.cursor.row,
                    self.cursor.attrs,
                    self.cursor.fg,
                    self.cursor.bg,
                ));
            }
            // DECRC - Restore Cursor
            'u' => {
                if let Some((col, row, attrs, fg, bg)) = self.saved_cursor {
                    self.cursor.col = col;
                    self.cursor.row = row;
                    self.cursor.attrs = attrs;
                    self.cursor.fg = fg;
                    self.cursor.bg = bg;
                }
                self.dirty = true;
            }
            // Hide/Show cursor
            'h' | 'l' => {
                if intermediates == b"?" {
                    let mode_set = action == 'h';
                    for &param in &params {
                        match param {
                            25 => self.cursor.visible = mode_set,
                            // Alt screen buffer
                            1049 => {
                                if mode_set {
                                    self.enter_alt_screen();
                                } else {
                                    self.exit_alt_screen();
                                }
                            }
                            _ => {}
                        }
                    }
                    self.dirty = true;
                }
            }
            // Device Status Report
            'n' => {
                // We handle DSR responses in the server
            }
            _ => {
                trace!("Unhandled CSI: {:?} {} {:?}", params, action, intermediates);
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            // DECSC - Save Cursor
            (_, b'7') => {
                self.saved_cursor = Some((
                    self.cursor.col,
                    self.cursor.row,
                    self.cursor.attrs,
                    self.cursor.fg,
                    self.cursor.bg,
                ));
            }
            // DECRC - Restore Cursor
            (_, b'8') => {
                if let Some((col, row, attrs, fg, bg)) = self.saved_cursor {
                    self.cursor.col = col;
                    self.cursor.row = row;
                    self.cursor.attrs = attrs;
                    self.cursor.fg = fg;
                    self.cursor.bg = bg;
                    self.dirty = true;
                }
            }
            // RI - Reverse Index
            (_, b'M') => {
                if self.cursor.row == self.scroll_top {
                    self.grid.scroll_down(self.scroll_top, self.scroll_bottom);
                } else if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                }
                self.dirty = true;
            }
            // IND - Index
            (_, b'D') => {
                self.line_feed();
                self.dirty = true;
            }
            // NEL - Next Line
            (_, b'E') => {
                self.cursor.col = 0;
                self.line_feed();
                self.dirty = true;
            }
            _ => {
                trace!("Unhandled ESC: {:?} 0x{:02x}", intermediates, byte);
            }
        }
    }
}
