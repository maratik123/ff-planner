mod inspector;
mod palette;

use crate::buildings::Category;
use egui::Color32;

pub(crate) fn category_color(category: Category) -> Color32 {
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
