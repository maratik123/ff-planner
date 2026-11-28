use std::sync::OnceLock;

use crate::app::TemplateApp;
use crate::buildings::all;

/// Sorted (id, label) entries for the palette, built once.
fn palette_entries() -> &'static Vec<(String, String)> {
    static ENTRIES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let mut flat: Vec<(String, String)> = all()
            .iter()
            .map(|(id, b)| (id.clone(), format!("{id}  ({:?})", b.category)))
            .collect();
        flat.sort_by(|a, b| a.0.cmp(&b.0));
        flat
    })
}

impl TemplateApp {
    pub(crate) fn palette_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Palette");
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, label) in palette_entries() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_entries_are_sorted_and_non_empty() {
        let entries = palette_entries();
        assert_ne!(entries.as_slice(), []);
        let mut sorted = entries.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(*entries, sorted);
    }

    #[test]
    fn palette_entries_are_cached() {
        let first = std::ptr::from_ref(palette_entries());
        let second = std::ptr::from_ref(palette_entries());
        assert_eq!(first, second);
    }
}
