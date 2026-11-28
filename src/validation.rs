use std::collections::{HashMap, HashSet};

use crate::buildings::{Prerequisite, all};
use crate::placement::{Layout, TilePos};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    Overlap,
    WithoutPrerequisite,
    ExceedsMax,
    UnknownBuilding,
}

impl IssueKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overlap => "Overlap",
            Self::WithoutPrerequisite => "Without prerequisite",
            Self::ExceedsMax => "Exceeds max allowed",
            Self::UnknownBuilding => "Unknown building",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub kind: IssueKind,
    pub instance_id: u64,
    pub detail: String,
}

fn overlap_issues(layout: &Layout) -> Vec<Issue> {
    let cache = crate::placement::OccupiedCells::new(layout);
    let mut seen_cells: HashMap<TilePos, u64> = HashMap::new();
    let mut overlap_cells: HashMap<(u64, u64), Vec<TilePos>> = HashMap::new();
    for b in &layout.buildings {
        if !all().contains_key(&b.building_identifier) {
            continue;
        }
        for cell in cache.occupied_cells(b).iter().copied() {
            if let Some(other_id) = seen_cells.get(&cell) {
                if *other_id != b.instance_id {
                    overlap_cells
                        .entry((b.instance_id, *other_id))
                        .or_default()
                        .push(cell);
                }
            } else {
                seen_cells.insert(cell, b.instance_id);
            }
        }
    }
    let mut overlap_pairs: Vec<(u64, u64, Vec<TilePos>)> = overlap_cells
        .into_iter()
        .map(|((id, other), mut cells)| {
            cells.sort_by_key(|c| (c.y, c.x));
            (id, other, cells)
        })
        .collect();
    overlap_pairs.sort_by_key(|(id, other, _)| (*id, *other));
    overlap_pairs
        .into_iter()
        .map(|(id, other, cells)| {
            let cell_list = cells
                .iter()
                .map(|c| format!("({}, {})", c.x, c.y))
                .collect::<Vec<_>>()
                .join(", ");
            Issue {
                kind: IssueKind::Overlap,
                instance_id: id,
                detail: format!("cells {cell_list} also used by instance {other}"),
            }
        })
        .collect()
}

fn prereq_issues(layout: &Layout, identifiers: &HashSet<String>) -> Vec<Issue> {
    let mut issues = Vec::new();
    for b in &layout.buildings {
        let Some(def) = all().get(&b.building_identifier) else {
            continue;
        };
        let mut prereqs: Vec<&Prerequisite> = def.prerequisites.iter().collect();
        prereqs.sort_by_key(|p| p.to_string());
        for prereq in prereqs {
            // Skip prereqs with no known building identifiers (e.g. Tech_NNN entries).
            let has_known = prereq
                .alternatives
                .iter()
                .any(|alt| all().contains_key(alt));
            if !has_known {
                continue;
            }
            if !prereq.is_satisfied(identifiers) {
                issues.push(Issue {
                    kind: IssueKind::WithoutPrerequisite,
                    instance_id: b.instance_id,
                    detail: format!("requires {prereq}"),
                });
            }
        }
    }
    issues
}

fn max_issues(layout: &Layout) -> Vec<Issue> {
    let mut counts: HashMap<&str, Vec<u64>> = HashMap::new();
    for b in &layout.buildings {
        if let Some(def) = all().get(&b.building_identifier)
            && def.enforce_max_allowed
        {
            counts
                .entry(b.building_identifier.as_str())
                .or_default()
                .push(b.instance_id);
        }
    }
    let mut counts_list: Vec<(&str, &Vec<u64>)> = counts.iter().map(|(k, v)| (*k, v)).collect();
    counts_list.sort_by_key(|(k, _)| *k);
    let mut issues = Vec::new();
    for (identifier, ids) in counts_list {
        let Some(def) = all().get(identifier) else {
            continue;
        };
        let max = def.max_allowed;
        // Sort by position (not instance id) so the flagged set does not depend
        // on creation order; overflow instances are the tail.
        let mut ids = ids.clone();
        ids.sort_by_key(|id| {
            let b = layout.get(*id);
            b.map(|b| (b.pos.y, b.pos.x, b.instance_id))
                .unwrap_or((i32::MAX, i32::MAX, *id))
        });
        for instance_id in ids.into_iter().skip(max as usize) {
            issues.push(Issue {
                kind: IssueKind::ExceedsMax,
                instance_id,
                detail: format!("{identifier} (max {max})"),
            });
        }
    }
    issues
}

pub fn validate_layout(layout: &Layout) -> Vec<Issue> {
    let mut issues = Vec::new();
    let identifiers: HashSet<String> = layout
        .buildings
        .iter()
        .map(|b| b.building_identifier.clone())
        .collect();

    for b in &layout.buildings {
        if !all().contains_key(&b.building_identifier) {
            issues.push(Issue {
                kind: IssueKind::UnknownBuilding,
                instance_id: b.instance_id,
                detail: b.building_identifier.clone(),
            });
        }
    }
    issues.extend(overlap_issues(layout));
    issues.extend(prereq_issues(layout, &identifiers));
    issues.extend(max_issues(layout));
    issues
}

impl Layout {
    pub fn validate(&self) -> Vec<Issue> {
        validate_layout(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::all;
    use crate::placement::{PlacedBuilding, Rotation};

    fn add(layout: &mut Layout, identifier: &str, x: i32, y: i32) -> u64 {
        layout
            .add(identifier, TilePos { x, y }, Rotation::Zero)
            .expect("add should succeed")
    }

    #[test]
    fn empty_layout_has_no_issues() {
        let layout = Layout::default();
        assert_eq!(layout.validate(), vec![]);
    }

    #[test]
    fn overlap_detected() {
        let mut layout = Layout::default();
        add(&mut layout, "Academy", 0, 0);
        let id = {
            // Force an overlapping placement bypassing can_place checks.
            layout.buildings.push(PlacedBuilding {
                instance_id: layout.next_instance_id,
                building_identifier: "School".to_owned(),
                pos: TilePos { x: 1, y: 0 },
                rotation: Rotation::Zero,
            });
            layout.next_instance_id += 1;
            layout
                .buildings
                .last()
                .expect("pushed building")
                .instance_id
        };
        let issues = layout.validate();
        let overlaps: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::Overlap && i.instance_id == id)
            .collect();
        assert_eq!(overlaps.len(), 1);
    }

    #[test]
    fn missing_prerequisite_detected() {
        let mut layout = Layout::default();
        // School requires TownCenter_Tier2 (not placed).
        let id = add(&mut layout, "School", 0, 0);
        let issues = layout.validate();
        let missing: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::WithoutPrerequisite && i.instance_id == id)
            .collect();
        assert_eq!(missing.len(), 1);
        assert!(missing[0].detail.contains("TownCenter_Tier2"));
    }

    #[test]
    fn satisfied_prerequisites_produce_no_issue() {
        let mut layout = Layout::default();
        add(&mut layout, "TownCenter_Tier2", 0, 0);
        let id = add(&mut layout, "School", 20, 0);
        let issues = layout.validate();
        assert!(!issues.iter().any(|i| i.instance_id == id));
    }

    #[test]
    fn or_prerequisite_satisfied_by_either() {
        let mut layout = Layout::default();
        // Cheesemaker requires (Barn|GoatBarn) AND Tech_085. Barn exists in the dump;
        // GoatBarn and Tech_085 do not, so only the Barn alternative is checkable
        // and Tech_085 is skipped entirely.
        add(&mut layout, "Barn", 0, 0);
        let id = add(&mut layout, "Cheesemaker", 60, 0);
        let issues = layout.validate();
        assert!(!issues.iter().any(|i| i.instance_id == id));
    }

    #[test]
    fn unknown_prerequisite_alternatives_are_skipped() {
        let mut layout = Layout::default();
        // BookBinder's only prerequisite, Tech_090, is absent from the dump: no issue.
        let id = add(&mut layout, "BookBinder", 0, 0);
        let issues = layout.validate();
        assert!(!issues.iter().any(|i| i.instance_id == id));
    }

    #[test]
    fn or_prerequisite_unsatisfied_when_barn_and_goat_barn_absent() {
        // GoatBarn is also absent from the dump; Barn alone is the only known alternative.
        let mut layout = Layout::default();
        let id = add(&mut layout, "Cheesemaker", 60, 0);
        let issues = layout.validate();
        let missing: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::WithoutPrerequisite && i.instance_id == id)
            .collect();
        assert_eq!(missing.len(), 1);
        assert!(missing[0].detail.contains("Barn"));
    }

    #[test]
    fn enforced_max_exceeded_detected() {
        let mut layout = Layout::default();
        add(&mut layout, "TownCenter", 0, 0);
        // Force a second TownCenter bypassing can_place (occupied cells differ? just force).
        layout.buildings.push(PlacedBuilding {
            instance_id: layout.next_instance_id,
            building_identifier: "TownCenter".to_owned(),
            pos: TilePos { x: 100, y: 0 },
            rotation: Rotation::Zero,
        });
        layout.next_instance_id += 1;
        let issues = layout.validate();
        let over: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::ExceedsMax)
            .collect();
        assert_eq!(over.len(), 1);
    }

    #[test]
    fn unknown_building_detected() {
        let mut layout = Layout::default();
        layout.buildings.push(PlacedBuilding {
            instance_id: 0,
            building_identifier: "NoSuchBuilding".to_owned(),
            pos: TilePos { x: 0, y: 0 },
            rotation: Rotation::Zero,
        });
        layout.next_instance_id = 1;
        let issues = layout.validate();
        assert!(issues.iter().any(|i| i.kind == IssueKind::UnknownBuilding));
    }

    #[test]
    fn overlap_issue_lists_all_colliding_cells() {
        let mut layout = Layout::default();
        add(&mut layout, "Academy", 0, 0);
        let id = {
            layout.buildings.push(PlacedBuilding {
                instance_id: layout.next_instance_id,
                building_identifier: "School".to_owned(),
                pos: TilePos { x: 4, y: 0 },
                rotation: Rotation::Zero,
            });
            layout.next_instance_id += 1;
            layout.buildings.last().expect("pushed").instance_id
        };
        let issues = layout.validate();
        let overlaps: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == IssueKind::Overlap && i.instance_id == id)
            .collect();
        assert_eq!(overlaps.len(), 1);
        assert!(overlaps[0].detail.contains("(4, 0)"));
        assert!(overlaps[0].detail.contains("(4, 1)"));
        assert!(overlaps[0].detail.contains("(4, 2)"));
    }

    #[test]
    fn exceeds_max_flagged_positions_independent_of_creation_order() {
        let mut layout_a = Layout::default();
        add(&mut layout_a, "TownCenter", 0, 0);
        layout_a.buildings.push(PlacedBuilding {
            instance_id: layout_a.next_instance_id,
            building_identifier: "TownCenter".to_owned(),
            pos: TilePos { x: 100, y: 0 },
            rotation: Rotation::Zero,
        });
        layout_a.next_instance_id += 1;

        let mut layout_b = Layout::default();
        layout_b.buildings.push(PlacedBuilding {
            instance_id: 0,
            building_identifier: "TownCenter".to_owned(),
            pos: TilePos { x: 100, y: 0 },
            rotation: Rotation::Zero,
        });
        layout_b.next_instance_id = 1;
        layout_b.buildings.push(PlacedBuilding {
            instance_id: 1,
            building_identifier: "TownCenter".to_owned(),
            pos: TilePos { x: 0, y: 0 },
            rotation: Rotation::Zero,
        });
        layout_b.next_instance_id = 2;

        let flagged = |l: &Layout| {
            let mut v: Vec<TilePos> = l
                .validate()
                .into_iter()
                .filter(|i| i.kind == IssueKind::ExceedsMax)
                .map(|i| l.get(i.instance_id).expect("flagged building").pos)
                .collect();
            v.sort_by_key(|p| (p.y, p.x));
            v
        };
        assert_eq!(flagged(&layout_a), flagged(&layout_b));
        assert_eq!(flagged(&layout_a), vec![TilePos { x: 100, y: 0 }]);
    }

    #[test]
    fn defs_used_in_tests_exist() {
        let all = all();
        assert!(all.get("Academy").is_some());
        assert!(all.get("School").is_some());
        assert!(all.get("TownCenter_Tier2").is_some());
        assert!(all.get("Barn").is_some());
        assert!(all.get("Cheesemaker").is_some());
        assert!(all.get("TownCenter").is_some());
    }
}
