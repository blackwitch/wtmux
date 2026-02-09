use serde::{Deserialize, Serialize};

/// A rectangle representing the position and size of a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the right edge (exclusive).
    pub fn right(&self) -> u16 {
        self.x + self.width
    }

    /// Returns the bottom edge (exclusive).
    pub fn bottom(&self) -> u16 {
        self.y + self.height
    }

    /// Check if a point is inside this rect.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Shrink the rect by a border on all sides.
    pub fn inset(&self, border: u16) -> Rect {
        Rect {
            x: self.x + border,
            y: self.y + border,
            width: self.width.saturating_sub(border * 2),
            height: self.height.saturating_sub(border * 2),
        }
    }
}
