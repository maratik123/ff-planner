use egui::Vec2;

use crate::canvas::DragState;
use crate::history::LayoutHistory;
use crate::placement::Layout;

/// Current schema version of the persisted app state.
pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Saves written before versioning existed deserialize as this; `migrate` lifts them.
fn legacy_schema_version() -> u32 {
    0
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    /// 0 for saves written before versioning existed; `migrate` lifts it.
    #[serde(default = "legacy_schema_version")]
    pub(crate) schema_version: u32,
    pub(crate) layout: Layout,
    pub(crate) selected: Option<u64>,
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
            selected: None,
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
        if let Some(id) = self.selected
            && self.layout.get(id).is_none()
        {
            self.selected = None;
        }
    }

    fn redo(&mut self) {
        if !self.history.redo(&mut self.layout) {
            return;
        }
        if let Some(id) = self.selected
            && self.layout.get(id).is_none()
        {
            self.selected = None;
        }
    }

    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        let mods = ui.input(|i| i.modifiers);
        let z = ui.input(|i| i.key_pressed(egui::Key::Z));
        let y = ui.input(|i| i.key_pressed(egui::Key::Y));
        if mods.command && z && mods.shift {
            self.redo();
        } else if mods.command && (z || y) {
            self.undo();
        }
    }

    /// Lift a loaded save up to the current schema version. Returns true if any migration ran.
    fn migrate(&mut self) -> bool {
        if self.schema_version >= CURRENT_SCHEMA_VERSION {
            return false;
        }
        // No field changes yet; this is the hook for future migrations.
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
        let json = r#"{"schema_version":1,"zoom":1.5}"#;
        let mut app: TemplateApp = serde_json::from_str(json).expect("parse save");
        assert!(!app.migrate());
        assert_eq!(app.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn default_app_has_current_schema_version() {
        let app = TemplateApp::default();
        assert_eq!(app.schema_version, CURRENT_SCHEMA_VERSION);
    }
}
