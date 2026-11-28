use super::category_color;
use crate::app::TemplateApp;
use crate::buildings::{Category, all};
use crate::history::Command;
use crate::validation::IssueKind;

impl TemplateApp {
    pub(crate) fn inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();
        self.selected_panel(ui);
        ui.separator();
        ui.heading("Issues");
        self.issues_panel(ui);
        ui.separator();
        ui.heading("Stats");
        self.stats_panel(ui);
    }

    fn single_selected_building(&self) -> Option<&crate::placement::PlacedBuilding> {
        let id = *self.selected.first()?;
        let b = self.layout.get(id)?;
        all().contains_key(&b.building_identifier).then_some(b)
    }

    fn selected_panel(&mut self, ui: &mut egui::Ui) {
        if self.selected.len() > 1 {
            let present = self
                .selected
                .iter()
                .filter(|id| self.layout.get(**id).is_some())
                .count();
            ui.strong(format!("{present} buildings selected"));
            ui.separator();
            if ui.button("Rotate").clicked()
                && let Some(cmd) = crate::selection::batch_rotate(&self.layout, &self.selected)
            {
                self.history.exec(cmd, &mut self.layout);
                crate::selection::prune_selection(&self.layout, &mut self.selected);
            }
            if ui.button("Delete").clicked()
                && let Some(cmd) = crate::selection::batch_delete(&self.layout, &self.selected)
            {
                self.history.exec(cmd, &mut self.layout);
                self.selected.clear();
            }
        } else if let Some(b) = self.single_selected_building().cloned() {
            if let Some(def) = all().get(&b.building_identifier) {
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
                        .exec(Command::Remove { placed: b }, &mut self.layout);
                    self.selected.clear();
                }
            } else {
                ui.label("No building selected.");
            }
        } else {
            ui.label("No building selected.");
        }
    }

    fn issues_panel(&self, ui: &mut egui::Ui) {
        let issues = self.layout.validate();
        if issues.is_empty() {
            ui.label("No issues.");
        } else {
            ui.label(format!(
                "{} issue{}",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            ));
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for issue in &issues {
                        let color = match issue.kind {
                            IssueKind::Overlap => egui::Color32::from_rgb(0xff, 0, 0),
                            _ => egui::Color32::from_rgb(0xff, 0xa5, 0x00),
                        };
                        ui.colored_label(
                            color,
                            format!(
                                "{}: {} (instance {})",
                                issue.kind.label(),
                                issue.detail,
                                issue.instance_id
                            ),
                        );
                    }
                });
        }
    }

    fn stats_panel(&self, ui: &mut egui::Ui) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::{Rotation, TilePos};

    #[test]
    fn single_selected_building_none_when_missing() {
        let app = TemplateApp {
            selected: vec![999],
            ..Default::default()
        };
        assert!(app.single_selected_building().is_none());
    }

    #[test]
    fn single_selected_building_some_when_present() {
        let mut app = TemplateApp::default();
        let id = app
            .layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        app.selected = vec![id]; // selection set after layout mutation
        assert!(app.single_selected_building().is_some());
    }

    #[test]
    fn single_selected_building_none_when_empty() {
        let app = TemplateApp::default();
        assert!(app.single_selected_building().is_none());
    }
}
