pub mod geometry;

use geometry::Rect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Pane ID type (matches wtmux-common::PaneId internally).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub Uuid);

/// Split orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Tree-based layout node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Leaf(PaneId),
    Split {
        orientation: Orientation,
        children: Vec<LayoutNode>,
        ratios: Vec<f32>,
    },
}

impl LayoutNode {
    /// Create a new leaf node.
    pub fn leaf(pane_id: PaneId) -> Self {
        LayoutNode::Leaf(pane_id)
    }

    /// Split this node, returning the new pane's position.
    pub fn split(&mut self, new_pane: PaneId, orientation: Orientation) {
        let old = std::mem::replace(self, LayoutNode::Leaf(new_pane));
        *self = LayoutNode::Split {
            orientation,
            children: vec![old, LayoutNode::Leaf(new_pane)],
            ratios: vec![0.5, 0.5],
        };
    }

    /// Split a specific pane within the tree.
    pub fn split_pane(
        &mut self,
        target: PaneId,
        new_pane: PaneId,
        orientation: Orientation,
    ) -> bool {
        match self {
            LayoutNode::Leaf(id) if *id == target => {
                self.split(new_pane, orientation);
                true
            }
            LayoutNode::Leaf(_) => false,
            LayoutNode::Split {
                children,
                orientation: split_orient,
                ratios,
            } => {
                // First try to find the target in children
                for (i, child) in children.iter_mut().enumerate() {
                    if let LayoutNode::Leaf(id) = child {
                        if *id == target {
                            if *split_orient == orientation {
                                // Same orientation: add as sibling
                                let new_ratio = ratios[i] / 2.0;
                                ratios[i] = new_ratio;
                                children.insert(i + 1, LayoutNode::Leaf(new_pane));
                                ratios.insert(i + 1, new_ratio);
                                return true;
                            } else {
                                // Different orientation: replace leaf with sub-split
                                child.split(new_pane, orientation);
                                return true;
                            }
                        }
                    }
                }
                // Recurse into children
                for child in children.iter_mut() {
                    if child.split_pane(target, new_pane, orientation) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Remove a pane from the layout. Returns true if found and removed.
    pub fn remove_pane(&mut self, target: PaneId) -> bool {
        match self {
            LayoutNode::Leaf(_) => false,
            LayoutNode::Split {
                children,
                ratios,
                ..
            } => {
                // Find and remove the target
                if let Some(idx) = children.iter().position(|child| {
                    matches!(child, LayoutNode::Leaf(id) if *id == target)
                }) {
                    children.remove(idx);
                    let removed_ratio = ratios.remove(idx);

                    // Redistribute ratio
                    if !ratios.is_empty() {
                        let bonus = removed_ratio / ratios.len() as f32;
                        for r in ratios.iter_mut() {
                            *r += bonus;
                        }
                    }

                    // If only one child remains, collapse
                    if children.len() == 1 {
                        let remaining = children.remove(0);
                        *self = remaining;
                    }

                    return true;
                }

                // Recurse
                for child in children.iter_mut() {
                    if child.remove_pane(target) {
                        // Check if child collapsed to a leaf
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Calculate the geometry (position & size) of each pane.
    pub fn calculate_geometries(&self, area: Rect) -> HashMap<PaneId, Rect> {
        let mut result = HashMap::new();
        self.calc_geo_inner(area, &mut result);
        result
    }

    fn calc_geo_inner(&self, area: Rect, result: &mut HashMap<PaneId, Rect>) {
        match self {
            LayoutNode::Leaf(pane_id) => {
                result.insert(*pane_id, area);
            }
            LayoutNode::Split {
                orientation,
                children,
                ratios,
            } => {
                let mut offset = 0u16;
                let total = match orientation {
                    Orientation::Horizontal => area.width,
                    Orientation::Vertical => area.height,
                };

                for (i, (child, &ratio)) in children.iter().zip(ratios.iter()).enumerate() {
                    let size = if i == children.len() - 1 {
                        // Last child gets remaining space to avoid rounding gaps
                        total - offset
                    } else {
                        (total as f32 * ratio).round() as u16
                    };

                    let child_area = match orientation {
                        Orientation::Horizontal => Rect {
                            x: area.x + offset,
                            y: area.y,
                            width: size,
                            height: area.height,
                        },
                        Orientation::Vertical => Rect {
                            x: area.x,
                            y: area.y + offset,
                            width: area.width,
                            height: size,
                        },
                    };

                    child.calc_geo_inner(child_area, result);
                    offset += size;
                }
            }
        }
    }

    /// Get all pane IDs in this layout.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        self.collect_pane_ids(&mut ids);
        ids
    }

    fn collect_pane_ids(&self, ids: &mut Vec<PaneId>) {
        match self {
            LayoutNode::Leaf(id) => ids.push(*id),
            LayoutNode::Split { children, .. } => {
                for child in children {
                    child.collect_pane_ids(ids);
                }
            }
        }
    }

    /// Find the next pane in the given direction from the target pane.
    pub fn find_adjacent_pane(
        &self,
        target: PaneId,
        direction: Direction,
        area: Rect,
    ) -> Option<PaneId> {
        let geometries = self.calculate_geometries(area);
        let target_rect = geometries.get(&target)?;

        let target_center_x = target_rect.x + target_rect.width / 2;
        let target_center_y = target_rect.y + target_rect.height / 2;

        let mut best: Option<(PaneId, u32)> = None;

        for (&pane_id, rect) in &geometries {
            if pane_id == target {
                continue;
            }

            let center_x = rect.x + rect.width / 2;
            let center_y = rect.y + rect.height / 2;

            let is_valid = match direction {
                Direction::Left => center_x < target_center_x,
                Direction::Right => center_x > target_center_x,
                Direction::Up => center_y < target_center_y,
                Direction::Down => center_y > target_center_y,
            };

            if is_valid {
                let dist = match direction {
                    Direction::Left | Direction::Right => {
                        let dx = (center_x as i32 - target_center_x as i32).unsigned_abs();
                        let dy = (center_y as i32 - target_center_y as i32).unsigned_abs();
                        dx * 2 + dy
                    }
                    Direction::Up | Direction::Down => {
                        let dx = (center_x as i32 - target_center_x as i32).unsigned_abs();
                        let dy = (center_y as i32 - target_center_y as i32).unsigned_abs();
                        dy * 2 + dx
                    }
                };

                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((pane_id, dist));
                }
            }
        }

        best.map(|(id, _)| id)
    }

    /// Swap two pane IDs in the layout tree.
    pub fn swap_panes(&mut self, a: PaneId, b: PaneId) {
        self.swap_panes_inner(a, b);
    }

    fn swap_panes_inner(&mut self, a: PaneId, b: PaneId) {
        match self {
            LayoutNode::Leaf(id) => {
                if *id == a {
                    *id = b;
                } else if *id == b {
                    *id = a;
                }
            }
            LayoutNode::Split { children, .. } => {
                for child in children {
                    child.swap_panes_inner(a, b);
                }
            }
        }
    }

    /// Resize a pane by adjusting the split ratio of its parent.
    pub fn resize_pane(&mut self, target: PaneId, direction: Direction, amount: f32) -> bool {
        match self {
            LayoutNode::Leaf(_) => false,
            LayoutNode::Split {
                orientation,
                children,
                ratios,
            } => {
                // Find the target pane's index
                let target_idx = children.iter().position(|child| {
                    child.pane_ids().contains(&target)
                });

                if let Some(idx) = target_idx {
                    let should_resize = match (orientation, &direction) {
                        (Orientation::Horizontal, Direction::Left | Direction::Right) => true,
                        (Orientation::Vertical, Direction::Up | Direction::Down) => true,
                        _ => false,
                    };

                    if should_resize {
                        let grow = matches!(direction, Direction::Right | Direction::Down);
                        let neighbor_idx = if grow { idx + 1 } else { idx.wrapping_sub(1) };

                        if neighbor_idx < children.len() {
                            let delta = amount;
                            if grow {
                                ratios[idx] += delta;
                                ratios[neighbor_idx] -= delta;
                            } else {
                                ratios[idx] += delta;
                                ratios[neighbor_idx] -= delta;
                            }
                            // Clamp ratios
                            let min_ratio = 0.05;
                            for r in ratios.iter_mut() {
                                if *r < min_ratio {
                                    *r = min_ratio;
                                }
                            }
                            // Normalize
                            let sum: f32 = ratios.iter().sum();
                            for r in ratios.iter_mut() {
                                *r /= sum;
                            }
                            return true;
                        }
                    }

                    // Recurse into the child that contains the target
                    return children[idx].resize_pane(target, direction, amount);
                }

                false
            }
        }
    }
}

/// Direction for pane navigation and resize.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Built-in layout presets.
pub fn even_horizontal(pane_ids: &[PaneId], _area: Rect) -> LayoutNode {
    if pane_ids.len() == 1 {
        return LayoutNode::Leaf(pane_ids[0]);
    }
    let ratio = 1.0 / pane_ids.len() as f32;
    LayoutNode::Split {
        orientation: Orientation::Horizontal,
        children: pane_ids.iter().map(|&id| LayoutNode::Leaf(id)).collect(),
        ratios: vec![ratio; pane_ids.len()],
    }
}

pub fn even_vertical(pane_ids: &[PaneId], _area: Rect) -> LayoutNode {
    if pane_ids.len() == 1 {
        return LayoutNode::Leaf(pane_ids[0]);
    }
    let ratio = 1.0 / pane_ids.len() as f32;
    LayoutNode::Split {
        orientation: Orientation::Vertical,
        children: pane_ids.iter().map(|&id| LayoutNode::Leaf(id)).collect(),
        ratios: vec![ratio; pane_ids.len()],
    }
}

pub fn main_horizontal(pane_ids: &[PaneId], _area: Rect) -> LayoutNode {
    if pane_ids.len() == 1 {
        return LayoutNode::Leaf(pane_ids[0]);
    }
    let main_pane = LayoutNode::Leaf(pane_ids[0]);
    let others: Vec<LayoutNode> = pane_ids[1..].iter().map(|&id| LayoutNode::Leaf(id)).collect();
    let other_ratio = 1.0 / others.len() as f32;

    let bottom = if others.len() == 1 {
        others.into_iter().next().unwrap()
    } else {
        LayoutNode::Split {
            orientation: Orientation::Horizontal,
            children: others,
            ratios: vec![other_ratio; pane_ids.len() - 1],
        }
    };

    LayoutNode::Split {
        orientation: Orientation::Vertical,
        children: vec![main_pane, bottom],
        ratios: vec![0.6, 0.4],
    }
}

pub fn main_vertical(pane_ids: &[PaneId], _area: Rect) -> LayoutNode {
    if pane_ids.len() == 1 {
        return LayoutNode::Leaf(pane_ids[0]);
    }
    let main_pane = LayoutNode::Leaf(pane_ids[0]);
    let others: Vec<LayoutNode> = pane_ids[1..].iter().map(|&id| LayoutNode::Leaf(id)).collect();
    let other_ratio = 1.0 / others.len() as f32;

    let right = if others.len() == 1 {
        others.into_iter().next().unwrap()
    } else {
        LayoutNode::Split {
            orientation: Orientation::Vertical,
            children: others,
            ratios: vec![other_ratio; pane_ids.len() - 1],
        }
    };

    LayoutNode::Split {
        orientation: Orientation::Horizontal,
        children: vec![main_pane, right],
        ratios: vec![0.6, 0.4],
    }
}

pub fn tiled(pane_ids: &[PaneId], _area: Rect) -> LayoutNode {
    if pane_ids.len() <= 2 {
        return even_horizontal(pane_ids, _area);
    }

    // Arrange in a grid-like pattern
    let half = (pane_ids.len() + 1) / 2;
    let top_panes: Vec<LayoutNode> = pane_ids[..half].iter().map(|&id| LayoutNode::Leaf(id)).collect();
    let bottom_panes: Vec<LayoutNode> = pane_ids[half..].iter().map(|&id| LayoutNode::Leaf(id)).collect();

    let top_ratio = 1.0 / top_panes.len() as f32;
    let top = LayoutNode::Split {
        orientation: Orientation::Horizontal,
        children: top_panes,
        ratios: vec![top_ratio; half],
    };

    if bottom_panes.is_empty() {
        return top;
    }

    let bot_ratio = 1.0 / bottom_panes.len() as f32;
    let bottom = LayoutNode::Split {
        orientation: Orientation::Horizontal,
        children: bottom_panes,
        ratios: vec![bot_ratio; pane_ids.len() - half],
    };

    LayoutNode::Split {
        orientation: Orientation::Vertical,
        children: vec![top, bottom],
        ratios: vec![0.5, 0.5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pane_id() -> PaneId {
        PaneId(Uuid::new_v4())
    }

    #[test]
    fn test_leaf_geometry() {
        let pane = make_pane_id();
        let layout = LayoutNode::leaf(pane);
        let area = Rect::new(0, 0, 80, 24);
        let geos = layout.calculate_geometries(area);
        assert_eq!(geos[&pane], area);
    }

    #[test]
    fn test_horizontal_split() {
        let p1 = make_pane_id();
        let p2 = make_pane_id();
        let mut layout = LayoutNode::leaf(p1);
        layout.split_pane(p1, p2, Orientation::Horizontal);

        let area = Rect::new(0, 0, 80, 24);
        let geos = layout.calculate_geometries(area);
        assert_eq!(geos[&p1].width, 40);
        assert_eq!(geos[&p2].width, 40);
        assert_eq!(geos[&p1].x, 0);
        assert_eq!(geos[&p2].x, 40);
    }

    #[test]
    fn test_vertical_split() {
        let p1 = make_pane_id();
        let p2 = make_pane_id();
        let mut layout = LayoutNode::leaf(p1);
        layout.split_pane(p1, p2, Orientation::Vertical);

        let area = Rect::new(0, 0, 80, 24);
        let geos = layout.calculate_geometries(area);
        assert_eq!(geos[&p1].height, 12);
        assert_eq!(geos[&p2].height, 12);
    }

    #[test]
    fn test_remove_pane() {
        let p1 = make_pane_id();
        let p2 = make_pane_id();
        let mut layout = LayoutNode::leaf(p1);
        layout.split_pane(p1, p2, Orientation::Horizontal);
        assert!(layout.remove_pane(p2));
        assert_eq!(layout.pane_ids(), vec![p1]);
    }
}
