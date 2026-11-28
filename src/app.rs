use egui::Vec2;
use serde::Deserialize;

use crate::canvas::DragState;
use crate::history::LayoutHistory;
use crate::placement::Layout;
use crate::selection;

/// Current schema version of the persisted app state.
pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Saves written before versioning existed deserialize as this; `migrate` lifts them.
fn legacy_schema_version() -> u32 {
    0
}

/// Deserialize a legacy save's `selected` field: a number, `null`, or (v2) an array.
fn deserialize_selected<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        None_,
        One(u64),
        Many(Vec<u64>),
    }
    Ok(match Repr::deserialize(deserializer)? {
        Repr::None_ => Vec::new(),
        Repr::One(id) => vec![id],
        Repr::Many(ids) => ids,
    })
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    /// 0 for saves written before versioning existed; `migrate` lifts it.
    #[serde(default = "legacy_schema_version")]
    pub(crate) schema_version: u32,
    pub(crate) layout: Layout,
    #[serde(default, deserialize_with = "deserialize_selected")]
    pub(crate) selected: Vec<u64>,
    pub(crate) palette_selection: Option<String>,
    pub(crate) pan: Vec2,
    pub(crate) zoom: f32,
    #[serde(skip)]
    pub(crate) history: LayoutHistory,
    #[serde(skip)]
    pub(crate) drag: Option<DragState>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            layout: Layout::default(),
            selected: Vec::new(),
            palette_selection: None,
            pan: Vec2::ZERO,
            zoom: 1.0,
            history: LayoutHistory::default(),
            drag: None,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        app.migrate();
        app
    }

    fn undo(&mut self) {
        if !self.history.undo(&mut self.layout) {
            return;
        }
        selection::prune_selection(&self.layout, &mut self.selected);
    }

    fn redo(&mut self) {
        if !self.history.redo(&mut self.layout) {
            return;
        }
        selection::prune_selection(&self.layout, &mut self.selected);
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        let issues = self.layout.validate();
        ui.horizontal(|ui| {
            ui.label(format!("Buildings: {}", self.layout.buildings.len()));
            if !self.selected.is_empty() {
                ui.separator();
                ui.label(format!("Selected: {}", self.selected.len()));
            }
            ui.separator();
            if issues.is_empty() {
                ui.label("No validation issues");
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(0xff, 0xa5, 0x00),
                    format!(
                        "{} validation issue{}",
                        issues.len(),
                        if issues.len() == 1 { "" } else { "s" }
                    ),
                );
            }
        });
    }

    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        let mods = ui.input(|i| i.modifiers);
        let z = ui.input(|i| i.key_pressed(egui::Key::Z));
        let y = ui.input(|i| i.key_pressed(egui::Key::Y));
        let delete = ui.input(|i| i.key_pressed(egui::Key::Delete));
        let backspace = ui.input(|i| i.key_pressed(egui::Key::Backspace));
        if mods.command && z && mods.shift {
            self.redo();
        } else if mods.command && (z || y) {
            self.undo();
        } else if (delete || backspace)
            && self.drag.is_none()
            && !self.selected.is_empty()
            && let Some(cmd) = selection::batch_delete(&self.layout, &self.selected)
        {
            self.history.exec(cmd, &mut self.layout);
            self.selected.clear();
        }
    }

    /// Lift a loaded save up to the current schema version. Returns true if any migration ran.
    fn migrate(&mut self) -> bool {
        if self.schema_version >= CURRENT_SCHEMA_VERSION {
            return false;
        }
        // v1 -> v2: `selected` was Option<u64>; the custom deserializer turned
        // it into a Vec (empty, one-element, or already a list). Nothing else to lift.
        selection::prune_selection(&self.layout, &mut self.selected);
        self.schema_version = CURRENT_SCHEMA_VERSION;
        true
    }
}

impl eframe::App for TemplateApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ui);
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
        egui::Panel::bottom("status_bar")
            .resizable(false)
            .default_size(24.0)
            .show(ui, |ui| {
                self.status_bar(ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas_panel(ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_save_without_schema_version_migrates() {
        let json = r#"{"layout":{"buildings":[],"next_instance_id":3},"zoom":2.5}"#;
        let mut app: TemplateApp = serde_json::from_str(json).expect("parse old save");
        assert_eq!(app.schema_version, 0);
        assert!(app.migrate());
        assert_eq!(app.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(app.zoom, 2.5);
        assert_eq!(app.layout.next_instance_id, 3);
    }

    #[test]
    fn current_version_is_not_migrated() {
        let json = r#"{"schema_version":2,"zoom":1.5}"#;
        let mut app: TemplateApp = serde_json::from_str(json).expect("parse save");
        assert!(!app.migrate());
        assert_eq!(app.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn legacy_save_with_single_selected_lifts_to_vec() {
        let json =
            r#"{"schema_version":1,"layout":{"buildings":[],"next_instance_id":1},"selected":7}"#;
        let mut app: TemplateApp = serde_json::from_str(json).expect("parse save");
        assert_eq!(app.selected, vec![7]);
        assert!(app.migrate());
        assert_eq!(app.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(app.selected.is_empty(), "selection should be empty");
    }

    #[test]
    fn legacy_save_with_null_selected_lifts_to_empty() {
        let json = r#"{"schema_version":1,"layout":{"buildings":[],"next_instance_id":1},"selected":null}"#;
        let mut app: TemplateApp = serde_json::from_str(json).expect("parse save");
        assert!(app.selected.is_empty(), "selection should be empty");
        assert!(app.migrate());
    }

    #[test]
    fn default_app_has_current_schema_version() {
        let app = TemplateApp::default();
        assert_eq!(app.schema_version, CURRENT_SCHEMA_VERSION);
    }
}
