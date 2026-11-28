use egui::{Color32, CornerRadius, Pos2, Stroke, Vec2, vec2};

use crate::app::TemplateApp;
use crate::buildings::{Rect, all};
use crate::history::Command;
use crate::panels::category_color;
use crate::placement::{PlacedBuilding, Rotation, TilePos};

const CELL_PX: f32 = 16.0;

/// Normalize drag rotation steps into [0, 4).
fn next_rotation_steps(steps: u8, shift: bool) -> u8 {
    let delta: i32 = if shift { -1 } else { 1 };
    u8::try_from((i32::from(steps) + delta).rem_euclid(4)).unwrap_or(0)
}

/// Clamp a u32 grid dimension to a sane i32 for centering math.
fn grid_dim(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// Truncate a bounded grid coordinate to i32 (expects bounded values).
#[expect(clippy::cast_possible_truncation, reason = "bounded grid coordinates")]
fn to_i32(v: f32) -> i32 {
    v as i32
}
const DRAG_THRESHOLD_PX: f32 = 5.0;

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
fn drag_target_pos(orig_pos: TilePos, delta_px: Vec2, cell: f32) -> TilePos {
    TilePos {
        x: orig_pos.x + to_i32((delta_px.x / cell).round()),
        y: orig_pos.y + to_i32((delta_px.y / cell).round()),
    }
}

fn drag_delta(delta_px: Vec2, cell: f32) -> TilePos {
    drag_target_pos(TilePos { x: 0, y: 0 }, delta_px, cell)
}

fn rotation_changed(drag: &DragState) -> bool {
    drag.rotation_steps != 0
}

fn position_changed(drag: &DragState) -> bool {
    drag.delta != TilePos { x: 0, y: 0 }
}

/// Whether a finished drag should be committed as a single Batch command.
fn should_apply_drag(drag: &DragState) -> bool {
    drag.moved && (position_changed(drag) || rotation_changed(drag))
}

/// Whether every entry in a drag would be valid at its target (ignoring all
/// selected buildings, then checking they don't collide with each other).
fn valid_drag(layout: &crate::placement::Layout, drag: &DragState) -> bool {
    let ignore: std::collections::HashSet<u64> =
        drag.entries.iter().map(|e| e.instance_id).collect();
    let mut occupied: std::collections::HashSet<crate::placement::TilePos> =
        std::collections::HashSet::new();
    for e in &drag.entries {
        let Some(b) = layout.get(e.instance_id) else {
            return false;
        };
        let pos = drag.current_pos(e);
        let rotation = drag.current_rotation(e);
        if !layout.can_place_ignoring(&b.building_identifier, pos, rotation, &ignore) {
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
fn pending_move(layout: &crate::placement::Layout, drag: &DragState) -> Option<Command> {
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

impl TemplateApp {
    fn cell_size(&self) -> f32 {
        CELL_PX * self.zoom
    }

    fn origin(&self, rect: egui::Rect) -> Pos2 {
        rect.min + self.pan
    }

    #[expect(clippy::cast_precision_loss, reason = "bounded grid coordinates")]
    fn grid_to_screen(&self, rect: egui::Rect, pos: TilePos) -> Pos2 {
        self.origin(rect) + Vec2::new(pos.x as f32, pos.y as f32) * self.cell_size()
    }

    fn screen_to_grid(&self, rect: egui::Rect, screen: Pos2) -> TilePos {
        let cell = self.cell_size();
        let o = self.origin(rect);
        TilePos {
            x: to_i32(((screen.x - o.x) / cell).floor()),
            y: to_i32(((screen.y - o.y) / cell).floor()),
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "bounded grid coordinates")]
    fn building_rect(&self, rect: egui::Rect, b: &PlacedBuilding, dims: Rect) -> egui::Rect {
        let p = self.grid_to_screen(rect, b.pos);
        egui::Rect::from_min_size(
            p,
            vec2(
                dims.width as f32 * self.cell_size(),
                dims.height as f32 * self.cell_size(),
            ),
        )
    }

    fn paint_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let o = self.origin(rect);
        let cell = self.cell_size();
        let start_gx = to_i32(((rect.min.x - o.x) / cell).floor());
        let start_gy = to_i32(((rect.min.y - o.y) / cell).floor());
        let end_gx = to_i32(((rect.max.x - o.x) / cell).ceil());
        let end_gy = to_i32(((rect.max.y - o.y) / cell).ceil());
        let grid_color = Color32::from_gray(0x33);
        for gx in start_gx..=end_gx {
            let x = o.x + gx as f32 * cell;
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                Stroke::new(1.0, grid_color),
            );
        }
        for gy in start_gy..=end_gy {
            let y = o.y + gy as f32 * cell;
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                Stroke::new(1.0, grid_color),
            );
        }
    }

    fn paint_buildings(&self, painter: &egui::Painter, rect: egui::Rect) {
        for b in &self.layout.buildings {
            let Some(def) = all().get(&b.building_identifier) else {
                continue;
            };
            let dims = b.rotation.dimensions(def.grid);
            let br = self.building_rect(rect, b, dims);
            let mut color = category_color(def.category);
            let is_dragged = self.drag.as_ref().is_some_and(|d| {
                d.moved && d.entries.iter().any(|e| e.instance_id == b.instance_id)
            });
            if is_dragged {
                color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0x55);
            }
            let selected = self.selected.contains(&b.instance_id);
            painter.rect_filled(br, CornerRadius::ZERO, color);
            painter.rect_stroke(
                br,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Inside,
            );
            if selected {
                painter.rect_stroke(
                    br,
                    CornerRadius::ZERO,
                    Stroke::new(2.0, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
            }
            painter.text(
                br.center(),
                egui::Align2::CENTER_CENTER,
                &b.building_identifier,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }

    fn hit_building(&self, rect: egui::Rect, pos: Pos2) -> Option<u64> {
        self.layout.buildings.iter().rev().find_map(|b| {
            let def = all().get(&b.building_identifier)?;
            let dims = b.rotation.dimensions(def.grid);
            let br = self.building_rect(rect, b, dims);
            br.contains(pos).then_some(b.instance_id)
        })
    }

    fn paint_ghost(&self, painter: &egui::Painter, rect: egui::Rect) {
        let Some(drag) = &self.drag else { return };
        if !drag.moved {
            return;
        }
        let valid = valid_drag(&self.layout, drag);
        let color = if valid {
            Color32::from_rgba_unmultiplied(0x40, 0xd0, 0x40, 0x66)
        } else {
            Color32::from_rgba_unmultiplied(0xe0, 0x40, 0x40, 0x66)
        };
        let stroke = if valid {
            Color32::from_rgb(0x40, 0xd0, 0x40)
        } else {
            Color32::from_rgb(0xe0, 0x40, 0x40)
        };
        for e in &drag.entries {
            let Some(b) = self.layout.get(e.instance_id) else {
                continue;
            };
            let Some(def) = all().get(&b.building_identifier) else {
                continue;
            };
            let rotation = drag.current_rotation(e);
            let dims = rotation.dimensions(def.grid);
            let pos = drag.current_pos(e);
            let p = self.grid_to_screen(rect, pos);
            let br = egui::Rect::from_min_size(
                p,
                vec2(
                    dims.width as f32 * self.cell_size(),
                    dims.height as f32 * self.cell_size(),
                ),
            );
            painter.rect_filled(br, CornerRadius::ZERO, color);
            painter.rect_stroke(
                br,
                CornerRadius::ZERO,
                Stroke::new(2.0, stroke),
                egui::StrokeKind::Inside,
            );
        }
    }

    fn zoom_at_cursor(&mut self, rect: egui::Rect, cursor: Pos2, wheel: f32) {
        let old_cell = self.cell_size();
        let new_zoom = (self.zoom * (wheel * 0.0015).exp()).clamp(0.1, 8.0);
        let new_cell = CELL_PX * new_zoom;
        let o = self.origin(rect);
        let grid_x = (cursor.x - o.x) / old_cell;
        let grid_y = (cursor.y - o.y) / old_cell;
        self.zoom = new_zoom;
        let new_o_x = cursor.x - grid_x * new_cell;
        let new_o_y = cursor.y - grid_y * new_cell;
        self.pan = Vec2::new(new_o_x - rect.min.x, new_o_y - rect.min.y);
    }

    fn rotate_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        let Some(cmd) = crate::selection::batch_rotate(&self.layout, &self.selected) else {
            return;
        };
        self.history.exec(cmd, &mut self.layout);
    }

    fn place_at_click(&mut self, rect: egui::Rect, click: Pos2) {
        let grid = self.screen_to_grid(rect, click);
        let Some(sel) = self.palette_selection.clone() else {
            return;
        };
        let Some(def) = all().get(&sel) else { return };
        let place = TilePos {
            x: grid.x - grid_dim(def.grid.width) / 2,
            y: grid.y - grid_dim(def.grid.height) / 2,
        };
        if self.layout.can_place(&sel, place, Rotation::Zero, None) {
            let placed = PlacedBuilding {
                instance_id: self.layout.next_instance_id,
                building_identifier: sel.clone(),
                pos: place,
                rotation: Rotation::Zero,
            };
            self.selected = vec![placed.instance_id];
            self.history.exec(Command::Add { placed }, &mut self.layout);
        } else {
            self.selected.clear();
        }
    }

    fn handle_click(&mut self, ui: &egui::Ui, rect: egui::Rect, click: Pos2) {
        let hit = self.hit_building(rect, click);
        let modifiers = ui.input(|i| i.modifiers);
        if let Some(id) = hit {
            if modifiers.command {
                crate::selection::toggle_selection(&mut self.selected, id);
            } else if modifiers.shift {
                crate::selection::extend_selection_range(&self.layout, &mut self.selected, id);
            } else {
                self.selected = vec![id];
            }
            self.palette_selection = None;
        } else if self.palette_selection.is_some() {
            self.place_at_click(rect, click);
        } else {
            self.selected.clear();
        }
    }

    fn begin_drag(&mut self, rect: egui::Rect, pointer: Pos2) {
        let Some(id) = self.hit_building(rect, pointer) else {
            return;
        };
        if !self.selected.contains(&id) {
            self.selected = vec![id];
        }
        let entries: Vec<DragEntry> = self
            .selected
            .iter()
            .filter_map(|&sid| {
                self.layout.get(sid).cloned().map(|b| DragEntry {
                    instance_id: sid,
                    orig_pos: b.pos,
                    orig_rotation: b.rotation,
                })
            })
            .collect();
        if entries.is_empty() {
            return;
        }
        self.drag = Some(DragState {
            start_pointer: pointer,
            entries,
            delta: TilePos { x: 0, y: 0 },
            rotation_steps: 0,
            moved: false,
        });
    }

    fn update_drag(&mut self, pointer: Pos2) {
        let cell = self.cell_size();
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let delta_px = pointer - drag.start_pointer;
        if delta_px.length() >= DRAG_THRESHOLD_PX {
            drag.moved = true;
        }
        if drag.moved {
            drag.delta = drag_delta(delta_px, cell);
        }
    }

    fn finish_drag(&mut self) {
        let Some(drag) = self.drag.take() else { return };
        let Some(cmd) = pending_move(&self.layout, &drag) else {
            return;
        };
        self.history.exec(cmd, &mut self.layout);
        // Selection persists; ids may have been pruned already.
        crate::selection::prune_selection(&self.layout, &mut self.selected);
    }

    fn handle_drag_rotation(&mut self, ui: &egui::Ui) {
        let r = ui.input(|i| i.key_pressed(egui::Key::R));
        if !r {
            return;
        }
        let shift = ui.input(|i| i.modifiers.shift);
        if let Some(drag) = self.drag.as_mut() {
            drag.rotation_steps = next_rotation_steps(drag.rotation_steps, shift);
            ui.ctx().request_repaint();
        } else {
            self.rotate_selected();
        }
    }

    pub(crate) fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_gray(0x1e));
        self.paint_grid(&painter, rect);
        self.paint_buildings(&painter, rect);
        self.paint_ghost(&painter, rect);

        let wheel = response.ctx.input(|i| i.smooth_scroll_delta.y);
        if wheel != 0.0 && response.hovered() {
            let cursor = response.hover_pos().unwrap_or_else(|| rect.center());
            self.zoom_at_cursor(rect, cursor, wheel);
            ui.ctx().request_repaint();
        }

        if response.drag_started() {
            let pointer = response
                .interact_pointer_pos()
                .unwrap_or_else(|| rect.center());
            self.begin_drag(rect, pointer);
        }

        if response.dragged() {
            if self.drag.is_some() {
                let pointer = response
                    .interact_pointer_pos()
                    .unwrap_or_else(|| rect.center());
                self.update_drag(pointer);
            } else {
                self.pan += response.drag_delta();
            }
            ui.ctx().request_repaint();
        }

        if response.drag_stopped() {
            let cell = self.cell_size();
            let pointer = response
                .interact_pointer_pos()
                .unwrap_or_else(|| rect.center());
            if let Some(drag) = self.drag.as_mut() {
                let delta_px = pointer - drag.start_pointer;
                if delta_px.length() >= DRAG_THRESHOLD_PX {
                    drag.moved = true;
                }
                if drag.moved {
                    drag.delta = drag_delta(delta_px, cell);
                }
            }
            self.finish_drag();
        }

        if response.clicked() {
            let click = response
                .interact_pointer_pos()
                .unwrap_or_else(|| rect.center());
            self.handle_click(ui, rect, click);
        }

        self.handle_drag_rotation(ui);
    }
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
        let mut layout = crate::placement::Layout::default();
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
        let mut layout = crate::placement::Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let d = drag_with(vec![entry(id, 0, 0)]);
        assert!(pending_move(&layout, &d).is_none());
    }

    #[test]
    fn pending_move_returns_batch_when_valid() {
        let mut layout = crate::placement::Layout::default();
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
        let mut layout = crate::placement::Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let b = layout
            .add("Academy", TilePos { x: 10, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let mut d = drag_with(vec![entry(a, 0, 0), entry(b, 10, 0)]);
        d.delta = TilePos { x: 0, y: 0 };
        d.rotation_steps = 0;
        // Original positions are valid despite selected uniqueness (b still overlaps nothing).
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
        let mut app = TemplateApp::default();
        let id = app
            .layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        let rect = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 200.0));
        // Academy is 5x4 cells, cell 16px: covers (0,0)-(80,64).
        assert_eq!(app.hit_building(rect, Pos2::new(10.0, 10.0)), Some(id));
        assert_eq!(app.hit_building(rect, Pos2::new(150.0, 150.0)), None);
    }

    #[test]
    fn drag_state_current_pos_applies_delta() {
        let mut d = drag_with(vec![entry(1, 2, 3)]);
        d.delta = TilePos { x: 1, y: -1 };
        assert_eq!(d.current_pos(&d.entries[0]), TilePos { x: 3, y: 2 });
    }
}
