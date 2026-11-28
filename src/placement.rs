use serde::{Deserialize, Serialize};

use crate::buildings::{Rect, all};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Rotation {
    #[default]
    Zero,
    R90,
    R180,
    R270,
}

impl Rotation {
    pub fn next(self) -> Self {
        match self {
            Self::Zero => Self::R90,
            Self::R90 => Self::R180,
            Self::R180 => Self::R270,
            Self::R270 => Self::Zero,
        }
    }

    pub fn dimensions(self, grid: Rect) -> Rect {
        match self {
            Self::Zero | Self::R180 => grid,
            Self::R90 | Self::R270 => Rect {
                width: grid.height,
                height: grid.width,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacedBuilding {
    pub instance_id: u64,
    pub building_identifier: String,
    pub pos: TilePos,
    pub rotation: Rotation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Layout {
    pub buildings: Vec<PlacedBuilding>,
    pub next_instance_id: u64,
}

impl Layout {
    pub fn occupied_cells(&self, building: &PlacedBuilding) -> Vec<TilePos> {
        let def = all().get(&building.building_identifier);
        let Some(def) = def else { return Vec::new() };
        let dims = building.rotation.dimensions(def.grid);
        let mut cells = Vec::new();
        for dy in 0..dims.height {
            for dx in 0..dims.width {
                cells.push(TilePos {
                    x: building.pos.x + dx.cast_signed(),
                    y: building.pos.y + dy.cast_signed(),
                });
            }
        }
        cells
    }

    pub fn can_place(
        &self,
        building_identifier: &str,
        pos: TilePos,
        rotation: Rotation,
        ignore_instance_id: Option<u64>,
    ) -> bool {
        let ignore: std::collections::HashSet<u64> = ignore_instance_id.into_iter().collect();
        self.can_place_ignoring(building_identifier, pos, rotation, &ignore)
    }

    pub fn can_place_ignoring(
        &self,
        building_identifier: &str,
        pos: TilePos,
        rotation: Rotation,
        ignore: &std::collections::HashSet<u64>,
    ) -> bool {
        let Some(def) = all().get(building_identifier) else {
            return false;
        };
        let dims = rotation.dimensions(def.grid);
        if dims.width == 0 || dims.height == 0 {
            return false;
        }
        let candidate_cells: Vec<TilePos> = (0..dims.height)
            .flat_map(|dy| {
                (0..dims.width).map(move |dx| TilePos {
                    x: pos.x + dx.cast_signed(),
                    y: pos.y + dy.cast_signed(),
                })
            })
            .collect();
        let occupied: std::collections::HashSet<TilePos> = self
            .buildings
            .iter()
            .filter(|b| !ignore.contains(&b.instance_id))
            .flat_map(|b| self.occupied_cells(b))
            .collect();
        candidate_cells.iter().all(|c| !occupied.contains(c))
    }

    pub fn add(
        &mut self,
        building_identifier: &str,
        pos: TilePos,
        rotation: Rotation,
    ) -> Option<u64> {
        if !self.can_place(building_identifier, pos, rotation, None) {
            return None;
        }
        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;
        self.buildings.push(PlacedBuilding {
            instance_id,
            building_identifier: building_identifier.to_owned(),
            pos,
            rotation,
        });
        Some(instance_id)
    }

    pub fn remove(&mut self, instance_id: u64) -> bool {
        let before = self.buildings.len();
        self.buildings.retain(|b| b.instance_id != instance_id);
        self.buildings.len() != before
    }

    pub fn get(&self, instance_id: u64) -> Option<&PlacedBuilding> {
        self.buildings.iter().find(|b| b.instance_id == instance_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_dimensions() {
        let grid = Rect {
            width: 5,
            height: 4,
        };
        assert_eq!(Rotation::Zero.dimensions(grid), grid);
        assert_eq!(Rotation::R180.dimensions(grid), grid);
        assert_eq!(
            Rotation::R90.dimensions(grid),
            Rect {
                width: 4,
                height: 5
            }
        );
        assert_eq!(
            Rotation::R270.dimensions(grid),
            Rect {
                width: 4,
                height: 5
            }
        );
    }

    #[test]
    fn rotation_serializes_as_enum_variant() {
        let json = serde_json::to_string(&Rotation::R90).expect("serialize");
        assert_eq!(json, "\"R90\"");
        let back: Rotation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, Rotation::R90);
    }

    #[test]
    fn occupied_cells_respects_rotation() {
        let layout = Layout::default();
        let b = PlacedBuilding {
            instance_id: 0,
            building_identifier: "Academy".to_owned(), // 5x4
            pos: TilePos { x: 1, y: 2 },
            rotation: Rotation::R90, // becomes 4x5
        };
        let cells = layout.occupied_cells(&b);
        assert_eq!(cells.len(), 20);
        assert!(cells.contains(&TilePos { x: 4, y: 6 }));
        assert!(cells.contains(&TilePos { x: 1, y: 6 }));
    }

    #[test]
    fn can_place_detects_overlap() {
        let mut layout = Layout::default();
        layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("first add");
        assert!(!layout.can_place("Academy", TilePos { x: 4, y: 0 }, Rotation::Zero, None));
        assert!(layout.can_place("Academy", TilePos { x: 5, y: 0 }, Rotation::Zero, None));
    }

    #[test]
    fn can_place_ignores_self() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add");
        assert!(layout.can_place("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero, Some(id)));
    }

    #[test]
    fn add_assigns_incrementing_ids() {
        let mut layout = Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add");
        let b = layout
            .add("Academy", TilePos { x: 10, y: 0 }, Rotation::Zero)
            .expect("add");
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(layout.next_instance_id, 2);
    }

    #[test]
    fn add_fails_when_occupied() {
        let mut layout = Layout::default();
        layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add");
        assert!(
            layout
                .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
                .is_none()
        );
        assert_eq!(layout.buildings.len(), 1);
    }

    #[test]
    fn remove_works() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add");
        assert!(layout.remove(id));
        assert!(layout.get(id).is_none());
        assert!(!layout.remove(id));
    }

    #[test]
    fn layout_serde_roundtrip() {
        let mut layout = Layout::default();
        layout
            .add("Academy", TilePos { x: 3, y: 4 }, Rotation::R270)
            .expect("add");
        let json = serde_json::to_string(&layout).expect("serialize");
        let back: Layout = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(layout, back);
    }
}
