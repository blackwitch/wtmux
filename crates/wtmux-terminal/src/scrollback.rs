use crate::cell::Cell;
use std::collections::VecDeque;

/// Ring buffer for scrollback history.
pub struct Scrollback {
    lines: VecDeque<Vec<Cell>>,
    max_lines: usize,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Scrollback {
            lines: VecDeque::new(),
            max_lines,
        }
    }

    /// Push a line into the scrollback buffer.
    pub fn push_line(&mut self, line: Vec<Cell>) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Get a line from the scrollback (0 = most recent).
    pub fn get_line(&self, offset: usize) -> Option<&Vec<Cell>> {
        if offset < self.lines.len() {
            self.lines.get(self.lines.len() - 1 - offset)
        } else {
            None
        }
    }

    /// Total number of lines in the scrollback.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the scrollback is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Clear the scrollback buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Iterate over all lines (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &Vec<Cell>> {
        self.lines.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollback_push_and_get() {
        let mut sb = Scrollback::new(100);
        sb.push_line(vec![Cell::new('A')]);
        sb.push_line(vec![Cell::new('B')]);
        assert_eq!(sb.get_line(0).unwrap()[0].ch, 'B');
        assert_eq!(sb.get_line(1).unwrap()[0].ch, 'A');
    }

    #[test]
    fn test_scrollback_max_lines() {
        let mut sb = Scrollback::new(2);
        sb.push_line(vec![Cell::new('A')]);
        sb.push_line(vec![Cell::new('B')]);
        sb.push_line(vec![Cell::new('C')]);
        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get_line(0).unwrap()[0].ch, 'C');
        assert_eq!(sb.get_line(1).unwrap()[0].ch, 'B');
    }
}
