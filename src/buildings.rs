use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

const RAW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/building_dump.json"
));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub width: u32,
    pub height: u32,
}

impl Rect {
    // Floored and clamped so the cast can only truncate unreachable huge values.
    #[expect(clippy::cast_possible_truncation)]
    fn from_floats(width: f32, height: f32) -> Self {
        Self {
            width: width.floor().max(0.0).min(u32::MAX as f32) as u32,
            height: height.floor().max(0.0).min(u32::MAX as f32) as u32,
        }
    }
}

impl<'de> Deserialize<'de> for Rect {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildGroup {
    Building,
    Wall,
    Gate,
    Bridge,
    Road,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildsiteClearingMode {
    ClearBeforeBuild,
    NoClear,
}

/// One prerequisite entry: one or more alternative identifiers (OR).
/// A single-element set is the Single case.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prerequisite {
    alternatives: Vec<String>,
}

impl Prerequisite {
    pub fn from_alternated(s: &str) -> Self {
        let mut alternatives: Vec<String> = s
            .split('|')
            .filter(|alt| !alt.is_empty())
            .map(str::to_owned)
            .collect();
        alternatives.sort();
        alternatives.dedup();
        Self { alternatives }
    }

    /// True if any alternative is present in the provided identifier set.
    pub fn is_satisfied(&self, identifiers: &HashSet<String>) -> bool {
        self.alternatives
            .iter()
            .any(|alt| identifiers.contains(alt))
    }

    /// Humans-readable form, e.g. "`Barn` or `GoatBarn`".
    pub fn label(&self) -> String {
        self.alternatives.join(" or ")
    }

    pub fn alternatives(&self) -> &[String] {
        &self.alternatives
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct Building {
    pub enforce_max_allowed: bool,
    pub max_allowed: u32,
    pub weight_to_catch_fire: u32,
    pub placeable_grid_type: PlaceableGridType,
    pub default_builders: u32,
    pub max_builders: u32,
    pub work_required_to_construct: u32,
    pub gold_required_to_relocate: u32,
    pub work_required_to_deconstruct: u32,
    pub clear_details_border_width: f32,
    pub buildsite_clearing_mode: BuildsiteClearingMode,
    pub build_group: BuildGroup,
    pub subject_to_reserves: bool,
    pub item_lifetime_modifier: f32,
    pub default_workers: u32,
    pub max_workers: u32,
    pub category: Category,
    pub highlight_similar_buildings_on_select: bool,
    pub materials_list_dirty: bool,
    pub grid: Rect,
    pub prerequisites: HashSet<Prerequisite>,
    pub materials: HashMap<String, u32>,
    pub materials_after_construction: HashMap<String, u32>,
    pub lower_tier_identifiers: HashSet<String>,
}

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
    prerequisites: Vec<String>,
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
            prerequisites: raw
                .prerequisites
                .into_iter()
                .map(|s| Prerequisite::from_alternated(&s))
                .collect(),
            materials: raw.materials,
            materials_after_construction: raw.materials_after_construction,
            lower_tier_identifiers: raw.lower_tier_identifiers,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildingDump {
    buildings: Vec<RawBuilding>,
}

/// # Panics
/// Panics if the embedded dump cannot be parsed.
pub fn all() -> &'static HashMap<String, Building> {
    static MAP: OnceLock<HashMap<String, Building>> = OnceLock::new();
    MAP.get_or_init(|| {
        let dump: BuildingDump =
            serde_json::from_str(RAW).expect("embedded building dump must parse");
        dump.buildings
            .into_iter()
            .map(|raw| {
                let identifier = raw.identifier.clone();
                (identifier, Building::from(raw))
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_dump() -> BuildingDump {
        serde_json::from_str(RAW).expect("embedded building dump must parse")
    }

    #[test]
    fn parses_without_failure() {
        assert!(!all().is_empty());
    }

    #[test]
    fn total_matches_number_of_buildings() {
        let value: serde_json::Value = serde_json::from_str(RAW).expect("valid dump json");
        let total = value["total"].as_u64().expect("total must be a number");
        let len = value["buildings"]
            .as_array()
            .expect("buildings must be an array")
            .len();
        assert_eq!(usize::try_from(total).expect("total must fit usize"), len);
        assert_eq!(all().len(), len);
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
            assert_eq!(
                raw.grid_w.fract(),
                0.0,
                "gridW of {} is fractional: {}",
                raw.identifier,
                raw.grid_w
            );
            assert_eq!(
                raw.grid_h.fract(),
                0.0,
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
        assert!(building.prerequisites.iter().any(|p| p.label() == "School"));
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
