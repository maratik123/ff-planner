//! Properties of buildings from the in-game dump (`data/building_dump.json`).
//!
//! The dump is embedded at compile time with [`include_str!`] and parsed lazily
//! on first access to [`all`]. All enum-like fields are mapped to Rust enums;
//! ordered/unordered collections are used where appropriate.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

/// Raw JSON embedded into the binary at compile time.
const RAW: &str = include_str!("../data/building_dump.json");

/// An area in grid cells, e.g. a building footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Width in cells.
    pub width: u32,
    /// Height in cells.
    pub height: u32,
}

impl Rect {
    /// Builds a `Rect` from float dimensions, truncating (flooring) them down
    /// to cell counts. Negative values become 0.
    ///
    /// The cast is intentional: after flooring, values are integer, non-negative;
    /// only values above `u32::MAX` could truncate, which cannot happen in practice.
    #[expect(clippy::cast_possible_truncation)]
    pub fn from_floats(width: f32, height: f32) -> Self {
        Self {
            width: width.floor().max(0.0).min(u32::MAX as f32) as u32,
            height: height.floor().max(0.0).min(u32::MAX as f32) as u32,
        }
    }
}

impl<'de> Deserialize<'de> for Rect {
    /// Builds a `Rect` from a JSON object `{"width": <number>, "height": <number>}`,
    /// flooring the float dimensions to cell counts.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            width: f32,
            height: f32,
        }
        let helper = Helper::deserialize(deserializer)?;
        Ok(Self::from_floats(helper.width, helper.height))
    }
}

/// What category a building belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    AmenitiesAndServices,
    ResourceProduction,
    FoodProduction,
    Defenses,
    Decorations,
    WallsAndRoads,
    None,
    Storage,
    Shelter,
}

/// The group a building belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildGroup {
    Building,
    Wall,
    Gate,
    Bridge,
    Road,
}

/// Whether a buildsite is cleared before a building is constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildsiteClearingMode {
    ClearBeforeBuild,
    NoClear,
}

/// The grid type a building can be placed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum PlaceableGridType {
    #[serde(rename = "StrategicPlanning")]
    StrategicPlanning,
    #[serde(rename = "Honey")]
    Honey,
    #[serde(rename = "None")]
    None,
    #[serde(rename = "Fodder")]
    Fodder,
    #[serde(rename = "Minerals")]
    Minerals,
    #[serde(rename = "Fertility")]
    Fertility,
    #[serde(rename = "InfiniteMinerals")]
    InfiniteMinerals,
    #[serde(rename = "Fish")]
    Fish,
    #[serde(rename = "Foraging")]
    Foraging,
    #[serde(rename = "GoatFodder")]
    GoatFodder,
    #[serde(rename = "Hunting")]
    Hunting,
    #[serde(rename = "Water")]
    Water,
    #[serde(rename = "WorkCamp")]
    WorkCamp,
}

/// A building from the dump. Order of neighbors is not meaningful; the
/// identifier is the outer `HashMap` key, not a field of this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Building {
    /// Whether a max-allowed limit is enforced.
    pub enforce_max_allowed: bool,
    /// Maximum number of this building allowed.
    pub max_allowed: u32,
    /// Weight used when catching fire.
    pub weight_to_catch_fire: u32,
    /// The grid type this building can be placed on.
    pub placeable_grid_type: PlaceableGridType,
    /// Default number of builders.
    pub default_builders: u32,
    /// Maximum number of builders.
    pub max_builders: u32,
    /// Work needed to construct this building.
    pub work_required_to_construct: u32,
    /// Gold needed to relocate this building.
    pub gold_required_to_relocate: u32,
    /// Work needed to deconstruct this building.
    pub work_required_to_deconstruct: u32,
    /// Border width of the details inset in the UI.
    pub clear_details_border_width: f32,
    /// How the buildsite is cleared before construction.
    pub buildsite_clearing_mode: BuildsiteClearingMode,
    /// The group the building belongs to.
    pub build_group: BuildGroup,
    /// Whether this building is subject to reserves.
    pub subject_to_reserves: bool,
    /// Modifier applied to item lifetime.
    pub item_lifetime_modifier: f32,
    /// Default number of workers.
    pub default_workers: u32,
    /// Maximum number of workers.
    pub max_workers: u32,
    /// Category of the building.
    pub category: Category,
    /// Whether similar buildings are highlighted when this one is selected.
    pub highlight_similar_buildings_on_select: bool,
    /// Whether the materials list was marked dirty.
    pub materials_list_dirty: bool,
    /// Footprint size in cells (`grid_w` x `grid_h`).
    pub grid: Rect,
    /// Buildings required to be built/learned before this building.
    pub prerequisites: HashSet<String>,
    /// Materials needed to construct this building.
    pub materials: HashMap<String, u32>,
    /// Materials produced right after construction.
    pub materials_after_construction: HashMap<String, u32>,
    /// Identifiers of the lower tiers this building is related to.
    pub lower_tier_identifiers: HashSet<String>,
}

/// Wrapper used only for parsing the dump; the identifier is used as the
/// `HashMap` key in [`all`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBuilding {
    identifier: String,
    enforce_max_allowed: bool,
    max_allowed: u32,
    weight_to_catch_fire: u32,
    placeable_grid_type: PlaceableGridType,
    default_builders: u32,
    max_builders: u32,
    work_required_to_construct: u32,
    gold_required_to_relocate: u32,
    work_required_to_deconstruct: u32,
    clear_details_border_width: f32,
    buildsite_clearing_mode: BuildsiteClearingMode,
    build_group: BuildGroup,
    subject_to_reserves: bool,
    item_lifetime_modifier: f32,
    default_workers: u32,
    max_workers: u32,
    category: Category,
    highlight_similar_buildings_on_select: bool,
    materials_list_dirty: bool,
    grid_w: f32,
    grid_h: f32,
    #[serde(default)]
    prerequisites: HashSet<String>,
    materials: HashMap<String, u32>,
    #[serde(default)]
    materials_after_construction: HashMap<String, u32>,
    #[serde(default)]
    lower_tier_identifiers: HashSet<String>,
}

impl From<RawBuilding> for Building {
    fn from(raw: RawBuilding) -> Self {
        Self {
            enforce_max_allowed: raw.enforce_max_allowed,
            max_allowed: raw.max_allowed,
            weight_to_catch_fire: raw.weight_to_catch_fire,
            placeable_grid_type: raw.placeable_grid_type,
            default_builders: raw.default_builders,
            max_builders: raw.max_builders,
            work_required_to_construct: raw.work_required_to_construct,
            gold_required_to_relocate: raw.gold_required_to_relocate,
            work_required_to_deconstruct: raw.work_required_to_deconstruct,
            clear_details_border_width: raw.clear_details_border_width,
            buildsite_clearing_mode: raw.buildsite_clearing_mode,
            build_group: raw.build_group,
            subject_to_reserves: raw.subject_to_reserves,
            item_lifetime_modifier: raw.item_lifetime_modifier,
            default_workers: raw.default_workers,
            max_workers: raw.max_workers,
            category: raw.category,
            highlight_similar_buildings_on_select: raw.highlight_similar_buildings_on_select,
            materials_list_dirty: raw.materials_list_dirty,
            grid: Rect::from_floats(raw.grid_w, raw.grid_h),
            prerequisites: raw.prerequisites,
            materials: raw.materials,
            materials_after_construction: raw.materials_after_construction,
            lower_tier_identifiers: raw.lower_tier_identifiers,
        }
    }
}

/// Raw dump structure used only for parsing; not exposed to callers.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildingDump {
    total: u32,
    buildings: Vec<RawBuilding>,
}

/// Returns all buildings indexed by their identifier.
///
/// The embedded dump is parsed on first call and cached in a `OnceLock`.
///
/// # Panics
///
/// Panics if the embedded dump cannot be parsed. This is a compile-time-known
/// artifact, so a failure here means the dump is corrupted at build time.
pub fn all() -> &'static HashMap<String, Building> {
    static MAP: OnceLock<HashMap<String, Building>> = OnceLock::new();
    MAP.get_or_init(|| {
        let dump: BuildingDump =
            serde_json::from_str(RAW).expect("embedded building dump must parse");
        assert_eq!(
            dump.total as usize,
            dump.buildings.len(),
            "embedded dump: total is inconsistent with buildings"
        );
        dump.buildings
            .into_iter()
            .map(|building| {
                let identifier = building.identifier.clone();
                (identifier, Building::from(building))
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses the raw dump for structural validation in tests.
    fn raw_dump() -> BuildingDump {
        serde_json::from_str(RAW).expect("embedded building dump must parse")
    }

    #[test]
    fn parses_without_failure() {
        let map = all();
        assert!(!map.is_empty());
    }

    #[test]
    fn total_matches_number_of_buildings() {
        let dump = raw_dump();
        assert_eq!(dump.total as usize, dump.buildings.len());
        assert_eq!(all().len(), dump.buildings.len());
    }

    #[test]
    fn identifiers_are_unique() {
        let dump = raw_dump();
        let mut seen = HashSet::new();
        for raw in &dump.buildings {
            assert!(
                seen.insert(&raw.identifier),
                "duplicate identifier: {}",
                raw.identifier
            );
        }
    }

    #[test]
    fn grid_values_are_integer_valued() {
        let dump = raw_dump();
        for raw in &dump.buildings {
            assert!(
                raw.grid_w.fract() == 0.0,
                "gridW of {} is fractional: {}",
                raw.identifier,
                raw.grid_w
            );
            assert!(
                raw.grid_h.fract() == 0.0,
                "gridH of {} is fractional: {}",
                raw.identifier,
                raw.grid_h
            );
        }
    }

    #[test]
    fn known_building_decodes_correctly() {
        let building = all().get("Academy").expect("Academy must exist");
        assert_eq!(building.category, Category::AmenitiesAndServices);
        assert_eq!(building.build_group, BuildGroup::Building);
        assert_eq!(
            building.buildsite_clearing_mode,
            BuildsiteClearingMode::ClearBeforeBuild
        );
        assert_eq!(
            building.grid,
            Rect {
                width: 5,
                height: 4
            }
        );
        assert_eq!(building.materials.get("ItemPlanks"), Some(&50));
        assert!(building.prerequisites.contains("School"));
    }

    #[test]
    fn rect_floors_float_values() {
        let rect: Rect =
            serde_json::from_str(r#"{"width": 5.7, "height": 3.2}"#).expect("valid rect json");
        assert_eq!(
            rect,
            Rect {
                width: 5,
                height: 3
            }
        );
    }

    #[test]
    fn rect_from_floats_truncates_negatives_and_floors() {
        assert_eq!(
            Rect::from_floats(2.9, -1.0),
            Rect {
                width: 2,
                height: 0
            }
        );
    }
}
