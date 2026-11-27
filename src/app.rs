use egui::Vec2;

use crate::placement::Layout;

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
