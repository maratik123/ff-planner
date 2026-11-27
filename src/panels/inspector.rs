use super::category_color;
use crate::app::TemplateApp;
use crate::buildings::{Category, all};
use crate::history::Command;

impl TemplateApp {
    pub(crate) fn inspector_panel(&mut self, ui: &mut egui::Ui) {
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
                    self.history
                        .exec(Command::Remove { placed: b.clone() }, &mut self.layout);
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
}
