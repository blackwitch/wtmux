use anyhow::Result;
use std::collections::HashMap;
use wtmux_common::{PaneId, WindowId};
use wtmux_layout::{
    geometry::Rect, LayoutNode, Orientation,
    PaneId as LayoutPaneId,
};

use crate::pane::Pane;

/// A window contains one or more panes arranged in a layout.
pub struct Window {
    pub id: WindowId,
    pub name: String,
    pub index: usize,
    pub panes: HashMap<PaneId, Pane>,
    pub layout: LayoutNode,
    pub active_pane: PaneId,
    pub last_active_pane: Option<PaneId>,
    pub zoomed_pane: Option<PaneId>,
    layout_preset: usize,
    area: Rect,
}

impl Window {
    pub fn new(name: String, index: usize, first_pane: Pane, area: Rect) -> Self {
        let pane_id = first_pane.id;
        let layout_pane_id = to_layout_pane_id(pane_id);
        let mut panes = HashMap::new();
        panes.insert(pane_id, first_pane);

        Window {
            id: WindowId::new(),
            name,
            index,
            panes,
            layout: LayoutNode::leaf(layout_pane_id),
            active_pane: pane_id,
            last_active_pane: None,
            zoomed_pane: None,
            layout_preset: 0,
            area,
        }
    }

    /// Split the active pane.
    pub fn split_pane(&mut self, command: &str, horizontal: bool) -> Result<PaneId> {
        // Calculate the active pane's current geometry
        let geos = self.layout.calculate_geometries(self.pane_area());
        let active_geo = geos
            .get(&to_layout_pane_id(self.active_pane))
            .copied()
            .unwrap_or(self.pane_area());

        let orientation = if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        };

        // Calculate new pane size (half of current)
        let (cols, rows) = match orientation {
            Orientation::Horizontal => (active_geo.width / 2, active_geo.height),
            Orientation::Vertical => (active_geo.width, active_geo.height / 2),
        };

        let new_pane = Pane::new(command, cols.max(1), rows.max(1))?;
        let new_pane_id = new_pane.id;

        self.layout.split_pane(
            to_layout_pane_id(self.active_pane),
            to_layout_pane_id(new_pane_id),
            orientation,
        );

        self.panes.insert(new_pane_id, new_pane);
        self.last_active_pane = Some(self.active_pane);
        self.active_pane = new_pane_id;

        // Resize all panes to their new geometries
        self.apply_layout()?;

        Ok(new_pane_id)
    }

    /// Close a pane and remove it from the layout.
    pub fn close_pane(&mut self, pane_id: PaneId) -> bool {
        self.panes.remove(&pane_id);
        self.layout.remove_pane(to_layout_pane_id(pane_id));

        if self.active_pane == pane_id {
            // Select the first remaining pane
            if let Some(&id) = self.panes.keys().next() {
                self.active_pane = id;
            }
        }

        self.panes.is_empty()
    }

    /// Select pane in the given direction.
    pub fn select_pane_direction(&mut self, direction: wtmux_common::protocol::Direction) {
        let layout_dir = match direction {
            wtmux_common::protocol::Direction::Up => wtmux_layout::Direction::Up,
            wtmux_common::protocol::Direction::Down => wtmux_layout::Direction::Down,
            wtmux_common::protocol::Direction::Left => wtmux_layout::Direction::Left,
            wtmux_common::protocol::Direction::Right => wtmux_layout::Direction::Right,
        };

        if let Some(next) = self.layout.find_adjacent_pane(
            to_layout_pane_id(self.active_pane),
            layout_dir,
            self.pane_area(),
        ) {
            self.last_active_pane = Some(self.active_pane);
            self.active_pane = from_layout_pane_id(next);
        }
    }

    /// Toggle zoom on the active pane.
    pub fn toggle_zoom(&mut self) {
        self.zoomed_pane = if self.zoomed_pane.is_some() {
            None
        } else {
            Some(self.active_pane)
        };
    }

    /// Get the pane geometries, accounting for zoom.
    pub fn pane_geometries(&self) -> HashMap<PaneId, Rect> {
        if let Some(zoomed) = self.zoomed_pane {
            let mut map = HashMap::new();
            map.insert(zoomed, self.pane_area());
            map
        } else {
            self.layout
                .calculate_geometries(self.pane_area())
                .into_iter()
                .map(|(k, v)| (from_layout_pane_id(k), v))
                .collect()
        }
    }

    /// Resize the window area and update all pane sizes.
    pub fn resize(&mut self, area: Rect) -> Result<()> {
        self.area = area;
        self.apply_layout()
    }

    /// Apply the current layout, resizing all panes.
    fn apply_layout(&mut self) -> Result<()> {
        let geos = self.pane_geometries();
        for (pane_id, rect) in &geos {
            if let Some(pane) = self.panes.get_mut(pane_id) {
                let _ = pane.resize(rect.width.max(1), rect.height.max(1));
            }
        }
        Ok(())
    }

    /// Get the pane area (window area minus status bar).
    fn pane_area(&self) -> Rect {
        self.area
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn area_width(&self) -> u16 {
        self.area.width
    }

    pub fn area_height(&self) -> u16 {
        self.area.height
    }

    /// Select the last active pane (Ctrl-B ;).
    pub fn select_last_pane(&mut self) {
        if let Some(last) = self.last_active_pane {
            if self.panes.contains_key(&last) {
                let old = self.active_pane;
                self.active_pane = last;
                self.last_active_pane = Some(old);
            }
        }
    }

    /// Swap the active pane with an adjacent one in the given direction.
    pub fn swap_pane(&mut self, up: bool) -> Result<()> {
        let pane_ids = self.layout.pane_ids();
        if pane_ids.len() < 2 {
            return Ok(());
        }

        let active_layout_id = to_layout_pane_id(self.active_pane);
        let pos = pane_ids.iter().position(|id| *id == active_layout_id);
        if let Some(idx) = pos {
            let target_idx = if up {
                if idx == 0 { pane_ids.len() - 1 } else { idx - 1 }
            } else {
                if idx == pane_ids.len() - 1 { 0 } else { idx + 1 }
            };

            let other_id = pane_ids[target_idx];
            self.layout.swap_panes(active_layout_id, other_id);
            self.apply_layout()?;
        }
        Ok(())
    }

    /// Resize the active pane in the given direction.
    pub fn resize_pane_direction(
        &mut self,
        direction: wtmux_common::protocol::Direction,
        amount: u16,
    ) -> Result<()> {
        let layout_dir = match direction {
            wtmux_common::protocol::Direction::Up => wtmux_layout::Direction::Up,
            wtmux_common::protocol::Direction::Down => wtmux_layout::Direction::Down,
            wtmux_common::protocol::Direction::Left => wtmux_layout::Direction::Left,
            wtmux_common::protocol::Direction::Right => wtmux_layout::Direction::Right,
        };

        let total = match direction {
            wtmux_common::protocol::Direction::Up | wtmux_common::protocol::Direction::Down => {
                self.area.height
            }
            wtmux_common::protocol::Direction::Left | wtmux_common::protocol::Direction::Right => {
                self.area.width
            }
        };

        if total > 0 {
            let ratio_amount = amount as f32 / total as f32;
            self.layout.resize_pane(
                to_layout_pane_id(self.active_pane),
                layout_dir,
                ratio_amount,
            );
            self.apply_layout()?;
        }
        Ok(())
    }

    /// Cycle to the next layout preset (Ctrl-B Space).
    pub fn next_layout(&mut self) -> Result<()> {
        let pane_ids = self.layout.pane_ids();
        if pane_ids.len() < 2 {
            return Ok(());
        }

        const NUM_PRESETS: usize = 5;
        self.layout_preset = (self.layout_preset + 1) % NUM_PRESETS;

        self.layout = match self.layout_preset {
            0 => wtmux_layout::even_horizontal(&pane_ids, self.pane_area()),
            1 => wtmux_layout::even_vertical(&pane_ids, self.pane_area()),
            2 => wtmux_layout::main_horizontal(&pane_ids, self.pane_area()),
            3 => wtmux_layout::main_vertical(&pane_ids, self.pane_area()),
            _ => wtmux_layout::tiled(&pane_ids, self.pane_area()),
        };

        self.apply_layout()
    }

    /// Select the next pane in tree order (Ctrl-B o).
    pub fn select_next_pane(&mut self) {
        let pane_ids = self.layout.pane_ids();
        if pane_ids.len() < 2 {
            return;
        }
        let active_layout_id = to_layout_pane_id(self.active_pane);
        if let Some(idx) = pane_ids.iter().position(|id| *id == active_layout_id) {
            let next_idx = (idx + 1) % pane_ids.len();
            self.last_active_pane = Some(self.active_pane);
            self.active_pane = from_layout_pane_id(pane_ids[next_idx]);
        }
    }
}

fn to_layout_pane_id(id: PaneId) -> LayoutPaneId {
    LayoutPaneId(id.0)
}

fn from_layout_pane_id(id: LayoutPaneId) -> PaneId {
    PaneId(id.0)
}
