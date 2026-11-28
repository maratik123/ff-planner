mod geometry;

use egui::{Color32, CornerRadius, Pos2, Stroke, Vec2, vec2};

use crate::app::TemplateApp;
use crate::buildings::all;
use crate::history::Command;
use crate::panels::category_color;
use crate::placement::{PlacedBuilding, Rotation, TilePos};

pub(crate) use geometry::{DragEntry, DragState};

use geometry::{
    DRAG_THRESHOLD_PX, building_rect, cell_size, drag_delta, grid_dim, grid_to_screen,
    hit_building, next_rotation_steps, origin, pending_move, screen_to_grid, valid_drag,
};

impl TemplateApp {
    fn cell_size(&self) -> f32 {
        cell_size(self.zoom)
    }

    fn origin(&self, rect: egui::Rect) -> Pos2 {
        origin(rect, self.pan)
    }

    fn grid_to_screen(&self, rect: egui::Rect, pos: TilePos) -> Pos2 {
        grid_to_screen(rect, self.pan, self.zoom, pos)
    }

    fn screen_to_grid(&self, rect: egui::Rect, screen: Pos2) -> TilePos {
        screen_to_grid(rect, self.pan, self.zoom, screen)
    }

    fn building_rect(
        &self,
        rect: egui::Rect,
        b: &PlacedBuilding,
        dims: crate::buildings::Rect,
    ) -> egui::Rect {
        building_rect(rect, self.pan, self.zoom, b, dims)
    }

    fn paint_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let o = self.origin(rect);
        let cell = self.cell_size();
        let start_gx = geometry::to_i32(((rect.min.x - o.x) / cell).floor());
        let start_gy = geometry::to_i32(((rect.min.y - o.y) / cell).floor());
        let end_gx = geometry::to_i32(((rect.max.x - o.x) / cell).ceil());
        let end_gy = geometry::to_i32(((rect.max.y - o.y) / cell).ceil());
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
        hit_building(&self.layout, rect, self.pan, self.zoom, pos)
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
        let new_cell = geometry::CELL_PX * new_zoom;
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

    fn handle_hit_click(&mut self, modifiers: egui::Modifiers, id: u64) {
        if modifiers.command {
            crate::selection::toggle_selection(&mut self.selected, id);
        } else if modifiers.shift {
            crate::selection::extend_selection_range(&self.layout, &mut self.selected, id);
        } else {
            self.selected = vec![id];
        }
        self.palette_selection = None;
    }

    fn handle_place_click(&mut self, rect: egui::Rect, click: Pos2) {
        self.place_at_click(rect, click);
    }

    fn handle_clear_click(&mut self) {
        self.selected.clear();
    }

    fn handle_click(&mut self, ui: &egui::Ui, rect: egui::Rect, click: Pos2) {
        let hit = self.hit_building(rect, click);
        let modifiers = ui.input(|i| i.modifiers);
        if let Some(id) = hit {
            self.handle_hit_click(modifiers, id);
        } else if self.palette_selection.is_some() {
            self.handle_place_click(rect, click);
        } else {
            self.handle_clear_click();
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
