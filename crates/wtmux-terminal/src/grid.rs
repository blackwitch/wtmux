use crate::cell::Cell;

/// A 2D grid of cells representing the visible terminal area.
pub struct Grid {
    pub cols: u16,
    pub rows: u16,
    cells: Vec<Vec<Cell>>,
}

impl Grid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cells = (0..rows)
            .map(|_| vec![Cell::default(); cols as usize])
            .collect();
        Grid { cols, rows, cells }
    }

    /// Get a reference to a cell.
    pub fn cell(&self, col: u16, row: u16) -> &Cell {
        &self.cells[row as usize][col as usize]
    }

    /// Get a mutable reference to a cell.
    pub fn cell_mut(&mut self, col: u16, row: u16) -> &mut Cell {
        &mut self.cells[row as usize][col as usize]
    }

    /// Set a cell at the given position.
    pub fn set_cell(&mut self, col: u16, row: u16, cell: Cell) {
        if (col as usize) < self.cols as usize && (row as usize) < self.rows as usize {
            self.cells[row as usize][col as usize] = cell;
        }
    }

    /// Get a row as a slice.
    pub fn row(&self, row: u16) -> &[Cell] {
        &self.cells[row as usize]
    }

    /// Get a mutable row.
    pub fn row_mut(&mut self, row: u16) -> &mut Vec<Cell> {
        &mut self.cells[row as usize]
    }

    /// Scroll the grid up by one line (top line is lost, bottom line is blank).
    pub fn scroll_up(&mut self, top: u16, bottom: u16) {
        if top < bottom && bottom <= self.rows {
            self.cells.remove(top as usize);
            self.cells
                .insert(bottom as usize - 1, vec![Cell::default(); self.cols as usize]);
        }
    }

    /// Scroll the grid down by one line.
    pub fn scroll_down(&mut self, top: u16, bottom: u16) {
        if top < bottom && bottom <= self.rows {
            self.cells.remove(bottom as usize - 1);
            self.cells
                .insert(top as usize, vec![Cell::default(); self.cols as usize]);
        }
    }

    /// Clear a region of the grid.
    pub fn clear_region(&mut self, top: u16, left: u16, bottom: u16, right: u16) {
        for row in top..=bottom.min(self.rows - 1) {
            for col in left..=right.min(self.cols - 1) {
                self.cells[row as usize][col as usize] = Cell::default();
            }
        }
    }

    /// Clear the entire grid.
    pub fn clear(&mut self) {
        self.clear_region(0, 0, self.rows - 1, self.cols - 1);
    }

    /// Clear a single row.
    pub fn clear_row(&mut self, row: u16) {
        if (row as usize) < self.cells.len() {
            for cell in &mut self.cells[row as usize] {
                *cell = Cell::default();
            }
        }
    }

    /// Resize the grid, preserving content where possible.
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        // Adjust rows
        while self.cells.len() > new_rows as usize {
            self.cells.pop();
        }
        while self.cells.len() < new_rows as usize {
            self.cells.push(vec![Cell::default(); new_cols as usize]);
        }

        // Adjust columns
        for row in &mut self.cells {
            row.resize(new_cols as usize, Cell::default());
        }

        self.cols = new_cols;
        self.rows = new_rows;
    }

    /// Erase characters from cursor to end of line.
    pub fn erase_to_eol(&mut self, row: u16, col: u16) {
        if (row as usize) < self.cells.len() {
            for c in col..self.cols {
                self.cells[row as usize][c as usize] = Cell::default();
            }
        }
    }

    /// Extract the text content of a row as a string.
    pub fn row_text(&self, row: u16) -> String {
        if row >= self.rows {
            return String::new();
        }
        self.cells[row as usize]
            .iter()
            .filter(|c| c.width > 0)
            .map(|c| c.ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    /// Search for a string in the grid. Returns (col, row) of the first match
    /// starting from (start_col, start_row) in the given direction.
    pub fn search(
        &self,
        query: &str,
        start_col: u16,
        start_row: u16,
        forward: bool,
    ) -> Option<(u16, u16)> {
        if query.is_empty() {
            return None;
        }

        let query_lower = query.to_lowercase();

        if forward {
            // Search forward: from current position to end, then wrap to start
            for row in start_row..self.rows {
                let text = self.row_text(row);
                let text_lower = text.to_lowercase();
                let search_from = if row == start_row {
                    (start_col as usize).saturating_add(1).min(text.len())
                } else {
                    0
                };
                if let Some(pos) = text_lower[search_from..].find(&query_lower) {
                    return Some(((search_from + pos) as u16, row));
                }
            }
            // Wrap around
            for row in 0..=start_row.min(self.rows - 1) {
                let text = self.row_text(row);
                let text_lower = text.to_lowercase();
                let limit = if row == start_row {
                    start_col as usize
                } else {
                    text.len()
                };
                if let Some(pos) = text_lower[..limit.min(text.len())].find(&query_lower) {
                    return Some((pos as u16, row));
                }
            }
        } else {
            // Search backward
            for row in (0..=start_row.min(self.rows - 1)).rev() {
                let text = self.row_text(row);
                let text_lower = text.to_lowercase();
                let search_until = if row == start_row {
                    (start_col as usize).min(text.len())
                } else {
                    text.len()
                };
                if let Some(pos) = text_lower[..search_until].rfind(&query_lower) {
                    return Some((pos as u16, row));
                }
            }
            // Wrap around
            for row in (start_row.min(self.rows - 1)..self.rows).rev() {
                let text = self.row_text(row);
                let text_lower = text.to_lowercase();
                let from = if row == start_row {
                    (start_col as usize).saturating_add(1).min(text.len())
                } else {
                    0
                };
                if from < text.len() {
                    if let Some(pos) = text_lower[from..].rfind(&query_lower) {
                        return Some(((from + pos) as u16, row));
                    }
                }
            }
        }

        None
    }

    /// Erase characters from start of line to cursor.
    pub fn erase_to_bol(&mut self, row: u16, col: u16) {
        if (row as usize) < self.cells.len() {
            for c in 0..=col.min(self.cols - 1) {
                self.cells[row as usize][c as usize] = Cell::default();
            }
        }
    }

    /// Insert blank lines at the given row, pushing content down.
    pub fn insert_lines(&mut self, row: u16, count: u16, bottom: u16) {
        for _ in 0..count {
            if row < bottom && bottom <= self.rows {
                self.cells.remove(bottom as usize - 1);
                self.cells
                    .insert(row as usize, vec![Cell::default(); self.cols as usize]);
            }
        }
    }

    /// Delete lines at the given row, pulling content up.
    pub fn delete_lines(&mut self, row: u16, count: u16, bottom: u16) {
        for _ in 0..count {
            if row < bottom && bottom <= self.rows {
                self.cells.remove(row as usize);
                self.cells
                    .insert(bottom as usize - 1, vec![Cell::default(); self.cols as usize]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid() {
        let grid = Grid::new(80, 24);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.cell(0, 0).ch, ' ');
    }

    #[test]
    fn test_set_cell() {
        let mut grid = Grid::new(80, 24);
        grid.set_cell(5, 3, Cell::new('A'));
        assert_eq!(grid.cell(5, 3).ch, 'A');
    }

    #[test]
    fn test_scroll_up() {
        let mut grid = Grid::new(80, 3);
        grid.set_cell(0, 0, Cell::new('A'));
        grid.set_cell(0, 1, Cell::new('B'));
        grid.set_cell(0, 2, Cell::new('C'));
        grid.scroll_up(0, 3);
        assert_eq!(grid.cell(0, 0).ch, 'B');
        assert_eq!(grid.cell(0, 1).ch, 'C');
        assert_eq!(grid.cell(0, 2).ch, ' ');
    }

    #[test]
    fn test_resize() {
        let mut grid = Grid::new(80, 24);
        grid.set_cell(0, 0, Cell::new('X'));
        grid.resize(40, 12);
        assert_eq!(grid.cols, 40);
        assert_eq!(grid.rows, 12);
        assert_eq!(grid.cell(0, 0).ch, 'X');
    }
}
