use egui::{Pos2, Vec2};

use crate::buildings::{Rect, all};
use crate::history::Command;
use crate::placement::{Layout, PlacedBuilding, Rotation, TilePos};

pub(crate) const CELL_PX: f32 = 16.0;
pub(crate) const DRAG_THRESHOLD_PX: f32 = 5.0;

/// Normalize drag rotation steps into [0, 4).
pub(crate) fn next_rotation_steps(steps: u8, shift: bool) -> u8 {
    let delta: i32 = if shift { -1 } else { 1 };
    u8::try_from((i32::from(steps) + delta).rem_euclid(4)).unwrap_or(0)
}

/// Clamp a u32 grid dimension to a sane i32 for centering math.
pub(crate) fn grid_dim(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// Truncate a bounded grid coordinate to i32 (expects bounded values).
#[expect(clippy::cast_possible_truncation, reason = "bounded grid coordinates")]
pub(crate) fn to_i32(v: f32) -> i32 {
    v as i32
}

pub(crate) fn cell_size(zoom: f32) -> f32 {
    CELL_PX * zoom
}

pub(crate) fn origin(rect: egui::Rect, pan: Vec2) -> Pos2 {
    rect.min + pan
}

#[expect(clippy::cast_precision_loss, reason = "bounded grid coordinates")]
pub(crate) fn grid_to_screen(rect: egui::Rect, pan: Vec2, zoom: f32, pos: TilePos) -> Pos2 {
    origin(rect, pan) + Vec2::new(pos.x as f32, pos.y as f32) * cell_size(zoom)
}

pub(crate) fn screen_to_grid(rect: egui::Rect, pan: Vec2, zoom: f32, screen: Pos2) -> TilePos {
    let cell = cell_size(zoom);
    let o = origin(rect, pan);
    TilePos {
        x: to_i32(((screen.x - o.x) / cell).floor()),
        y: to_i32(((screen.y - o.y) / cell).floor()),
    }
}

#[expect(clippy::cast_precision_loss, reason = "bounded grid coordinates")]
pub(crate) fn building_rect(
    rect: egui::Rect,
    pan: Vec2,
    zoom: f32,
    b: &PlacedBuilding,
    dims: Rect,
) -> egui::Rect {
    let p = grid_to_screen(rect, pan, zoom, b.pos);
    egui::Rect::from_min_size(
        p,
        Vec2::new(
            dims.width as f32 * cell_size(zoom),
            dims.height as f32 * cell_size(zoom),
        ),
    )
}

/// One building captured at the start of a drag.
pub(crate) struct DragEntry {
    pub(crate) instance_id: u64,
    pub(crate) orig_pos: TilePos,
    pub(crate) orig_rotation: Rotation,
}

/// Transient state for an in-progress drag of a selection of buildings.
pub(crate) struct DragState {
    pub(crate) start_pointer: Pos2,
    pub(crate) entries: Vec<DragEntry>,
    pub(crate) delta: TilePos,
    pub(crate) rotation_steps: u8,
    pub(crate) moved: bool,
}

impl DragState {
    pub(crate) fn current_rotation(&self, entry: &DragEntry) -> Rotation {
        let mut r = entry.orig_rotation;
        for _ in 0..self.rotation_steps {
            r = r.next();
        }
        r
    }

    pub(crate) fn current_pos(&self, entry: &DragEntry) -> TilePos {
        TilePos {
            x: entry.orig_pos.x + self.delta.x,
            y: entry.orig_pos.y + self.delta.y,
        }
    }
}

/// Destination tile for a drag: the original position shifted by the pointer delta.
pub(crate) fn drag_target_pos(orig_pos: TilePos, delta_px: Vec2, cell: f32) -> TilePos {
    TilePos {
        x: orig_pos.x + to_i32((delta_px.x / cell).round()),
        y: orig_pos.y + to_i32((delta_px.y / cell).round()),
    }
}

pub(crate) fn drag_delta(delta_px: Vec2, cell: f32) -> TilePos {
    drag_target_pos(TilePos { x: 0, y: 0 }, delta_px, cell)
}

pub(crate) fn rotation_changed(drag: &DragState) -> bool {
    drag.rotation_steps != 0
}

pub(crate) fn position_changed(drag: &DragState) -> bool {
    drag.delta != TilePos { x: 0, y: 0 }
}

/// Whether a finished drag should be committed as a single Batch command.
pub(crate) fn should_apply_drag(drag: &DragState) -> bool {
    drag.moved && (position_changed(drag) || rotation_changed(drag))
}

/// Whether every entry in a drag would be valid at its target (ignoring all
/// selected buildings, then checking they don't collide with each other).
pub(crate) fn valid_drag(layout: &Layout, drag: &DragState) -> bool {
    let ignore: std::collections::HashSet<u64> =
        drag.entries.iter().map(|e| e.instance_id).collect();
    let cache = crate::placement::OccupiedCells::new(layout);
    let mut occupied: std::collections::HashSet<TilePos> = std::collections::HashSet::new();
    for e in &drag.entries {
        let Some(b) = layout.get(e.instance_id) else {
            return false;
        };
        let pos = drag.current_pos(e);
        let rotation = drag.current_rotation(e);
        if !cache.can_place_ignoring(&b.building_identifier, pos, rotation, &ignore) {
            return false;
        }
        let dims = rotation.dimensions(all()[&b.building_identifier].grid);
        for dy in 0..dims.height {
            for dx in 0..dims.width {
                let c = TilePos {
                    x: pos.x + dx.cast_signed(),
                    y: pos.y + dy.cast_signed(),
                };
                if !occupied.insert(c) {
                    return false;
                }
            }
        }
    }
    true
}

/// The undoable command for a finished drag, or `None` if the drop should be cancelled.
pub(crate) fn pending_move(layout: &Layout, drag: &DragState) -> Option<Command> {
    if !should_apply_drag(drag) {
        return None;
    }
    if !valid_drag(layout, drag) {
        return None;
    }
    let mut cmds = Vec::new();
    for e in &drag.entries {
        layout.get(e.instance_id)?;
        let to = drag.current_pos(e);
        let to_rot = drag.current_rotation(e);
        if to != e.orig_pos || to_rot != e.orig_rotation {
            cmds.push(Command::Move {
                instance_id: e.instance_id,
                from: e.orig_pos,
                to,
                from_rotation: e.orig_rotation,
                to_rotation: to_rot,
            });
        }
    }
    if cmds.is_empty() {
        None
    } else {
        Some(Command::Batch(cmds))
    }
}

pub(crate) fn hit_building(
    layout: &Layout,
    rect: egui::Rect,
    pan: Vec2,
    zoom: f32,
    pos: Pos2,
) -> Option<u64> {
    layout.buildings.iter().rev().find_map(|b| {
        let def = all().get(&b.building_identifier)?;
        let dims = b.rotation.dimensions(def.grid);
        let br = building_rect(rect, pan, zoom, b, dims);
        br.contains(pos).then_some(b.instance_id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, x: i32, y: i32) -> DragEntry {
        DragEntry {
            instance_id: id,
            orig_pos: TilePos { x, y },
            orig_rotation: Rotation::Zero,
        }
    }

    fn drag_with(entries: Vec<DragEntry>) -> DragState {
        DragState {
            start_pointer: Pos2::ZERO,
            entries,
            delta: TilePos { x: 0, y: 0 },
            rotation_steps: 0,
            moved: true,
        }
    }

    #[test]
    fn drag_target_pos_rounds_delta_to_nearest_cell() {
        let d = drag_target_pos(TilePos { x: 2, y: 2 }, Vec2::new(40.0, -30.0), 16.0);
        assert_eq!(d, TilePos { x: 5, y: 0 });
    }

    #[test]
    fn drag_target_pos_rounds_halves() {
        let d = drag_target_pos(TilePos { x: 0, y: 0 }, Vec2::new(12.0, 12.0), 16.0);
        assert_eq!(d, TilePos { x: 1, y: 1 });
    }

    #[test]
    fn drag_delta_rounds_to_nearest_cell() {
        let d = drag_delta(Vec2::new(40.0, -30.0), 16.0);
        assert_eq!(d, TilePos { x: 3, y: -2 });
    }

    #[test]
    fn should_apply_drag_false_when_not_moved() {
        let mut d = drag_with(vec![entry(1, 0, 0)]);
        d.moved = false;
        d.delta = TilePos { x: 5, y: 5 };
        assert!(!should_apply_drag(&d));
    }

    #[test]
    fn should_apply_drag_false_when_unchanged() {
        let d = drag_with(vec![entry(1, 0, 0)]);
        assert!(!should_apply_drag(&d));
    }

    #[test]
    fn should_apply_drag_true_on_position_change() {
        let mut d = drag_with(vec![entry(1, 0, 0)]);
        d.delta = TilePos { x: 1, y: 0 };
        assert!(should_apply_drag(&d));
    }

    #[test]
    fn should_apply_drag_true_on_rotation_change_only() {
        let mut d = drag_with(vec![entry(1, 0, 0)]);
        d.rotation_steps = 1;
        assert!(should_apply_drag(&d));
    }

    #[test]
    fn pending_move_returns_none_when_drop_invalid() {
        let mut layout = Layout::default();
        layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let id = layout.next_instance_id;
        // Reserve id 1 above; add a second building elsewhere.
        layout
            .add("Academy", TilePos { x: 0, y: 5 }, Rotation::Zero)
            .expect("add valid");
        let mut d = drag_with(vec![entry(id, 0, 5)]);
        d.delta = TilePos { x: 2, y: -3 }; // lands inside first building
        assert!(pending_move(&layout, &d).is_none());
    }

    #[test]
    fn pending_move_returns_none_without_change() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let d = drag_with(vec![entry(id, 0, 0)]);
        assert!(pending_move(&layout, &d).is_none());
    }

    #[test]
    fn pending_move_returns_batch_when_valid() {
        let mut layout = Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let b = layout
            .add("Academy", TilePos { x: 0, y: 5 }, Rotation::Zero)
            .expect("add valid");
        let mut d = drag_with(vec![entry(a, 0, 0), entry(b, 0, 5)]);
        d.delta = TilePos { x: 5, y: 0 };
        let cmd = pending_move(&layout, &d).expect("cmd");
        match cmd {
            Command::Batch(cmds) => {
                assert_eq!(cmds.len(), 2);
                assert!(matches!(cmds[0], Command::Move { instance_id, .. } if instance_id == a));
                assert!(matches!(cmds[1], Command::Move { instance_id, .. } if instance_id == b));
            }
            _ => panic!("expected Batch"),
        }
    }

    #[test]
    fn valid_drag_ignores_self_buildings() {
        let mut layout = Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let b = layout
            .add("Academy", TilePos { x: 10, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let mut d = drag_with(vec![entry(a, 0, 0), entry(b, 10, 0)]);
        d.delta = TilePos { x: 0, y: 0 };
        d.rotation_steps = 0;
        assert!(valid_drag(&layout, &d));
    }

    #[test]
    fn next_rotation_steps_wraps_around() {
        assert_eq!(next_rotation_steps(0, false), 1);
        assert_eq!(next_rotation_steps(3, false), 0);
        assert_eq!(next_rotation_steps(0, true), 3);
        assert_eq!(next_rotation_steps(3, true), 2);
    }

    #[test]
    fn grid_dim_clamps_huge_values() {
        assert_eq!(grid_dim(u32::MAX), i32::MAX);
        assert_eq!(grid_dim(5), 5);
    }

    #[test]
    fn hit_building_returns_topmost_building() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let rect = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 200.0));
        let pan = Vec2::ZERO;
        let zoom = 1.0;
        // Academy is 5x4 cells, cell 16px: covers (0,0)-(80,64).
        assert_eq!(
            hit_building(&layout, rect, pan, zoom, Pos2::new(10.0, 10.0)),
            Some(id)
        );
        assert_eq!(
            hit_building(&layout, rect, pan, zoom, Pos2::new(150.0, 150.0)),
            None
        );
    }

    #[test]
    fn drag_state_current_pos_applies_delta() {
        let mut d = drag_with(vec![entry(1, 2, 3)]);
        d.delta = TilePos { x: 1, y: -1 };
        assert_eq!(d.current_pos(&d.entries[0]), TilePos { x: 3, y: 2 });
    }
}
