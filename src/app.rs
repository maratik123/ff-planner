use crate::buildings::{Category, Rect, all};
use crate::placement::{Layout, PlacedBuilding, Rotation, TilePos};
use egui::{Color32, CornerRadius, Pos2, Stroke, Vec2, vec2};

const CELL_PX: f32 = 16.0;

fn category_color(category: Category) -> Color32 {
    match category {
        Category::AmenitiesAndServices => Color32::from_rgb(0x4f, 0xa6, 0xe0),
        Category::ResourceProduction => Color32::from_rgb(0xb0, 0x80, 0x50),
        Category::FoodProduction => Color32::from_rgb(0x6f, 0xb8, 0x4f),
        Category::Defenses => Color32::from_rgb(0xc0, 0x55, 0x55),
        Category::Decorations => Color32::from_rgb(0xa0, 0x70, 0xc0),
        Category::WallsAndRoads => Color32::from_rgb(0x90, 0x90, 0x90),
        Category::None => Color32::from_rgb(0x70, 0x70, 0x70),
        Category::Storage => Color32::from_rgb(0xcf, 0xa4, 0x4f),
        Category::Shelter => Color32::from_rgb(0x70, 0xc0, 0xb0),
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    pub(crate) layout: Layout,
    pub(crate) selected: Option<u64>,
    pub(crate) palette_selection: Option<String>,
    pub(crate) pan: Vec2,
    pub(crate) zoom: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            layout: Layout::default(),
            selected: None,
            palette_selection: None,
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }

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

    fn palette_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Palette");
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut flat: Vec<(String, String)> = all()
                .iter()
                .map(|(id, b)| (id.clone(), format!("{id}  ({:?})", b.category)))
                .collect();
            flat.sort_by(|a, b| a.0.cmp(&b.0));
            for (id, label) in flat {
                let is_selected = self.palette_selection.as_deref() == Some(id.as_str());
                let btn = egui::Button::new(label).selected(is_selected);
                if ui.add(btn).clicked() {
                    self.palette_selection = if is_selected { None } else { Some(id.clone()) };
                    self.selected = None;
                }
            }
        });
    }

    fn inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();
        if let Some(id) = self.selected {
            let building = self.layout.get(id);
            if let Some(b) = building.filter(|b| all().contains_key(&b.building_identifier))
                && let Some(def) = all().get(&b.building_identifier)
            {
                let dims = b.rotation.dimensions(def.grid);
                ui.horizontal(|ui| {
                    ui.colored_label(category_color(def.category), "■");
                    ui.strong(&b.building_identifier);
                });
                ui.label(format!("Category: {:?}", def.category));
                ui.label(format!("Size: {}x{} cells", dims.width, dims.height));
                ui.label(format!("Pos: ({}, {})", b.pos.x, b.pos.y));
                ui.label(format!("Rotation: {:?}", b.rotation));
                ui.label(format!(
                    "Builders: {}/{}",
                    def.default_builders, def.max_builders
                ));
                ui.label(format!(
                    "Work to construct: {}",
                    def.work_required_to_construct
                ));
                ui.separator();
                if ui.button("Remove").clicked() {
                    self.layout.remove(b.instance_id);
                    self.selected = None;
                }
            }
        } else {
            ui.label("No building selected.");
        }

        ui.separator();
        ui.heading("Stats");
        let mut by_category: std::collections::HashMap<Category, u32> =
            std::collections::HashMap::new();
        let mut total_cells = 0u32;
        let mut total = 0u32;
        for b in &self.layout.buildings {
            if let Some(def) = all().get(&b.building_identifier) {
                let dims = b.rotation.dimensions(def.grid);
                *by_category.entry(def.category).or_insert(0) += 1;
                total += 1;
                total_cells += dims.width * dims.height;
            }
        }
        let mut cat_list: Vec<(Category, u32)> = by_category.into_iter().collect();
        cat_list.sort_by_key(|(c, _)| format!("{c:?}"));
        ui.label(format!("Buildings: {total}"));
        ui.label(format!("Total cells: {total_cells}"));
        for (cat, count) in cat_list {
            ui.horizontal(|ui| {
                ui.colored_label(category_color(cat), "■");
                ui.label(format!("{cat:?}: {count}"));
            });
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
            let color = category_color(def.category);
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
            && let Some(b_mut) = self
                .layout
                .buildings
                .iter_mut()
                .find(|b| b.instance_id == id)
        {
            b_mut.rotation = new;
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
        if let Some(id) = self.layout.add(&sel, place, Rotation::Zero) {
            self.selected = Some(id);
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

    fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_gray(0x1e));
        self.paint_grid(&painter, rect);
        self.paint_buildings(&painter, rect);

        let wheel = response.ctx.input(|i| i.smooth_scroll_delta.y);
        if wheel != 0.0 && response.hovered() {
            let cursor = response.hover_pos().unwrap_or_else(|| rect.center());
            self.zoom_at_cursor(rect, cursor, wheel);
            ui.ctx().request_repaint();
        }

        if response.dragged() {
            self.pan += response.drag_delta();
            ui.ctx().request_repaint();
        }

        if response.clicked() {
            let click = response
                .interact_pointer_pos()
                .unwrap_or_else(|| rect.center());
            self.handle_click(rect, click);
        }

        if ui.input(|i| i.key_pressed(egui::Key::R)) {
            self.rotate_selected();
        }
    }
}

impl eframe::App for TemplateApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("palette_panel")
            .default_size(220.0)
            .show(ui, |ui| {
                self.palette_panel(ui);
            });
        egui::Panel::right("inspector_panel")
            .default_size(260.0)
            .show(ui, |ui| {
                self.inspector_panel(ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas_panel(ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
