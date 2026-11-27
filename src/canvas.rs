use egui::{Color32, CornerRadius, Pos2, Stroke, Vec2, vec2};

use crate::app::TemplateApp;
use crate::buildings::{Rect, all};
use crate::history::Command;
use crate::panels::category_color;
use crate::placement::{PlacedBuilding, Rotation, TilePos};

const CELL_PX: f32 = 16.0;
const DRAG_THRESHOLD_PX: f32 = 5.0;

/// Transient state for an in-progress drag of an existing building.
pub(crate) struct DragState {
    pub(crate) instance_id: u64,
    pub(crate) start_pointer: Pos2,
    pub(crate) orig_pos: TilePos,
    pub(crate) orig_rotation: Rotation,
    pub(crate) current_pos: TilePos,
    pub(crate) current_rotation: Rotation,
    pub(crate) moved: bool,
}

/// Destination tile for a drag: the original position shifted by the pointer delta.
#[expect(clippy::cast_possible_truncation, reason = "bounded grid coordinates")]
fn drag_target_pos(orig_pos: TilePos, delta_px: Vec2, cell: f32) -> TilePos {
    TilePos {
        x: orig_pos.x + (delta_px.x / cell).round() as i32,
        y: orig_pos.y + (delta_px.y / cell).round() as i32,
    }
}

fn rotation_changed(drag: &DragState) -> bool {
    drag.current_rotation != drag.orig_rotation
}

fn position_changed(drag: &DragState) -> bool {
    drag.current_pos != drag.orig_pos
}

/// Whether a finished drag should be committed as a single Move command.
fn should_apply_drag(drag: &DragState) -> bool {
    drag.moved && (position_changed(drag) || rotation_changed(drag))
}

/// The undoable command for a finished drag, or `None` if the drop should be cancelled.
fn pending_move(layout: &crate::placement::Layout, drag: &DragState) -> Option<Command> {
    if !should_apply_drag(drag) {
        return None;
    }
    let b = layout.get(drag.instance_id)?;
    if !layout.can_place(
        &b.building_identifier,
        drag.current_pos,
        drag.current_rotation,
        Some(drag.instance_id),
    ) {
        return None;
    }
    Some(Command::Move {
        instance_id: drag.instance_id,
        from: drag.orig_pos,
        to: drag.current_pos,
        from_rotation: drag.orig_rotation,
        to_rotation: drag.current_rotation,
    })
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

    #[expect(clippy::cast_possible_truncation, reason = "bounded grid coordinates")]
    fn screen_to_grid(&self, rect: egui::Rect, screen: Pos2) -> TilePos {
        let cell = self.cell_size();
        let o = self.origin(rect);
        TilePos {
            x: ((screen.x - o.x) / cell).floor() as i32,
            y: ((screen.y - o.y) / cell).floor() as i32,
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

    fn rotate_step(rotation: Rotation) -> Rotation {
        match rotation {
            Rotation::Zero => Rotation::R90,
            Rotation::R90 => Rotation::R180,
            Rotation::R180 => Rotation::R270,
            Rotation::R270 => Rotation::Zero,
        }
    }

    fn paint_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let o = self.origin(rect);
        let cell = self.cell_size();
        #[expect(clippy::cast_possible_truncation, reason = "bounded grid coordinates")]
        fn cell_at(v: f32) -> i32 {
            v as i32
        }
        let start_gx = cell_at(((rect.min.x - o.x) / cell).floor());
        let start_gy = cell_at(((rect.min.y - o.y) / cell).floor());
        let end_gx = cell_at(((rect.max.x - o.x) / cell).ceil());
        let end_gy = cell_at(((rect.max.y - o.y) / cell).ceil());
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
            let is_dragged = self
                .drag
                .as_ref()
                .is_some_and(|d| d.instance_id == b.instance_id && d.moved);
            if is_dragged {
                color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0x55);
            }
            let selected = self.selected == Some(b.instance_id);
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
        for b in self.layout.buildings.iter().rev() {
            let Some(def) = all().get(&b.building_identifier) else {
                continue;
            };
            let dims = b.rotation.dimensions(def.grid);
            let br = self.building_rect(rect, b, dims);
            if br.contains(pos) {
                return Some(b.instance_id);
            }
        }
        None
    }

    fn paint_ghost(&self, painter: &egui::Painter, rect: egui::Rect) {
        let Some(drag) = &self.drag else { return };
        if !drag.moved {
            return;
        }
        let Some(b) = self.layout.get(drag.instance_id) else {
            return;
        };
        let Some(def) = all().get(&b.building_identifier) else {
            return;
        };
        let valid = self.layout.can_place(
            &b.building_identifier,
            drag.current_pos,
            drag.current_rotation,
            Some(drag.instance_id),
        );
        let dims = drag.current_rotation.dimensions(def.grid);
        let p = self.grid_to_screen(rect, drag.current_pos);
        let br = egui::Rect::from_min_size(
            p,
            vec2(
                dims.width as f32 * self.cell_size(),
                dims.height as f32 * self.cell_size(),
            ),
        );
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
        painter.rect_filled(br, CornerRadius::ZERO, color);
        painter.rect_stroke(
            br,
            CornerRadius::ZERO,
            Stroke::new(2.0, stroke),
            egui::StrokeKind::Inside,
        );
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
        let Some(id) = self.selected else { return };
        let Some(b) = self.layout.get(id).cloned() else {
            return;
        };
        let new = Self::rotate_step(b.rotation);
        if self
            .layout
            .can_place(&b.building_identifier, b.pos, new, Some(b.instance_id))
        {
            self.history.exec(
                Command::Rotate {
                    instance_id: id,
                    from: b.rotation,
                    to: new,
                },
                &mut self.layout,
            );
        }
    }

    fn place_at_click(&mut self, rect: egui::Rect, click: Pos2) {
        let grid = self.screen_to_grid(rect, click);
        let Some(sel) = self.palette_selection.clone() else {
            return;
        };
        let Some(def) = all().get(&sel) else { return };
        let place = TilePos {
            x: grid.x - i32::try_from(def.grid.width).unwrap_or(0) / 2,
            y: grid.y - i32::try_from(def.grid.height).unwrap_or(0) / 2,
        };
        if self.layout.can_place(&sel, place, Rotation::Zero, None) {
            let placed = PlacedBuilding {
                instance_id: self.layout.next_instance_id,
                building_identifier: sel.clone(),
                pos: place,
                rotation: Rotation::Zero,
            };
            self.selected = Some(placed.instance_id);
            self.history.exec(Command::Add { placed }, &mut self.layout);
        } else {
            self.selected = None;
        }
    }

    fn handle_click(&mut self, rect: egui::Rect, click: Pos2) {
        let mut hit: Option<u64> = None;
        for b in self.layout.buildings.iter().rev() {
            let Some(def) = all().get(&b.building_identifier) else {
                continue;
            };
            let dims = b.rotation.dimensions(def.grid);
            let br = self.building_rect(rect, b, dims);
            if br.contains(click) {
                hit = Some(b.instance_id);
                break;
            }
        }
        if let Some(id) = hit {
            self.selected = Some(id);
            self.palette_selection = None;
        } else if self.palette_selection.is_some() {
            self.place_at_click(rect, click);
        } else {
            self.selected = None;
        }
    }

    fn begin_drag(&mut self, rect: egui::Rect, pointer: Pos2) {
        let Some(id) = self.hit_building(rect, pointer) else {
            return;
        };
        let Some(b) = self.layout.get(id).cloned() else {
            return;
        };
        self.drag = Some(DragState {
            instance_id: id,
            start_pointer: pointer,
            orig_pos: b.pos,
            orig_rotation: b.rotation,
            current_pos: b.pos,
            current_rotation: b.rotation,
            moved: false,
        });
    }

    fn update_drag(&mut self, pointer: Pos2) {
        let cell = self.cell_size();
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let delta = pointer - drag.start_pointer;
        if delta.length() >= DRAG_THRESHOLD_PX {
            drag.moved = true;
        }
        if drag.moved {
            drag.current_pos = drag_target_pos(drag.orig_pos, delta, cell);
        }
    }

    fn finish_drag(&mut self) {
        let Some(drag) = self.drag.take() else { return };
        let Some(cmd) = pending_move(&self.layout, &drag) else {
            return;
        };
        self.history.exec(cmd, &mut self.layout);
        self.selected = Some(drag.instance_id);
    }

    fn handle_drag_rotation(&mut self, ui: &egui::Ui) {
        let r = ui.input(|i| i.key_pressed(egui::Key::R));
        let shift = ui.input(|i| i.modifiers.shift);
        if !r && !shift {
            return;
        }
        if let Some(drag) = self.drag.as_mut() {
            drag.current_rotation = Self::rotate_step(drag.current_rotation);
            ui.ctx().request_repaint();
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
            if let Some(drag) = self.drag.as_mut() {
                let pointer = response
                    .interact_pointer_pos()
                    .unwrap_or_else(|| rect.center());
                let delta = pointer - drag.start_pointer;
                if delta.length() >= DRAG_THRESHOLD_PX {
                    drag.moved = true;
                }
                if drag.moved {
                    drag.current_pos = drag_target_pos(drag.orig_pos, delta, cell);
                }
            }
            self.finish_drag();
        }

        if response.clicked() {
            let click = response
                .interact_pointer_pos()
                .unwrap_or_else(|| rect.center());
            self.handle_click(rect, click);
        }

        self.handle_drag_rotation(ui);

        if ui.input(|i| i.key_pressed(egui::Key::R)) && self.drag.is_none() {
            self.rotate_selected();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag(orig_pos: TilePos, orig_rotation: Rotation) -> DragState {
        DragState {
            instance_id: 1,
            start_pointer: Pos2::ZERO,
            orig_pos,
            orig_rotation,
            current_pos: orig_pos,
            current_rotation: orig_rotation,
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
    fn should_apply_drag_false_when_not_moved() {
        let mut d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        d.moved = false;
        d.current_pos = TilePos { x: 5, y: 5 };
        assert!(!should_apply_drag(&d));
    }

    #[test]
    fn should_apply_drag_false_when_unchanged() {
        let d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        assert!(!should_apply_drag(&d));
    }

    #[test]
    fn should_apply_drag_true_on_position_change() {
        let mut d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        d.current_pos = TilePos { x: 1, y: 0 };
        assert!(should_apply_drag(&d));
    }

    #[test]
    fn pending_move_returns_none_when_drop_invalid() {
        let mut layout = crate::placement::Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        layout
            .add("Academy", TilePos { x: 0, y: 5 }, Rotation::Zero)
            .expect("unwrap");
        let mut d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        d.current_pos = TilePos { x: 2, y: 2 };
        d.instance_id = id;
        assert!(pending_move(&layout, &d).is_none());
    }

    #[test]
    fn pending_move_returns_none_without_change() {
        let mut layout = crate::placement::Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        let mut d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        d.instance_id = id;
        assert!(pending_move(&layout, &d).is_none());
    }

    #[test]
    fn pending_move_returns_move_command_when_valid() {
        let mut layout = crate::placement::Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        let mut d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        d.instance_id = id;
        d.current_pos = TilePos { x: 5, y: 5 };
        d.current_rotation = Rotation::R90;
        let cmd = pending_move(&layout, &d).expect("cmd");
        match cmd {
            Command::Move {
                instance_id,
                from,
                to,
                from_rotation,
                to_rotation,
            } => {
                assert_eq!(instance_id, id);
                assert_eq!(from, TilePos { x: 0, y: 0 });
                assert_eq!(to, TilePos { x: 5, y: 5 });
                assert_eq!(from_rotation, Rotation::Zero);
                assert_eq!(to_rotation, Rotation::R90);
            }
            _ => panic!("expected Move"),
        }
    }

    #[test]
    fn should_apply_drag_true_on_rotation_change_only() {
        let mut d = drag(TilePos { x: 0, y: 0 }, Rotation::Zero);
        d.current_rotation = Rotation::R90;
        assert!(should_apply_drag(&d));
    }
}
