use crate::app::TemplateApp;
use crate::buildings::all;

impl TemplateApp {
    pub(crate) fn palette_panel(&mut self, ui: &mut egui::Ui) {
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
                    self.selected.clear();
                }
            }
        });
    }
}
