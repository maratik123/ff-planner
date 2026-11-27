use std::collections::{HashMap, HashSet};

use crate::buildings::all;
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

fn prerequisite_satisfied(prereq: &str, identifiers: &HashSet<String>) -> bool {
    // '|' separates alternative identifiers within one prerequisite entry.
    prereq.split('|').any(|alt| identifiers.contains(alt))
}

pub fn validate_layout(layout: &Layout) -> Vec<Issue> {
    let mut issues = Vec::new();
    let identifiers: HashSet<String> = layout
        .buildings
        .iter()
        .map(|b| b.building_identifier.clone())
        .collect();

    let mut seen_cells: HashMap<TilePos, u64> = HashMap::new();
    for b in &layout.buildings {
        if !all().contains_key(&b.building_identifier) {
            issues.push(Issue {
                kind: IssueKind::UnknownBuilding,
                instance_id: b.instance_id,
                detail: b.building_identifier.clone(),
            });
            continue;
        }
        for cell in layout.occupied_cells(b) {
            if let Some(other_id) = seen_cells.get(&cell) {
                issues.push(Issue {
                    kind: IssueKind::Overlap,
                    instance_id: b.instance_id,
                    detail: format!(
                        "cell ({}, {}) also used by instance {}",
                        cell.x, cell.y, other_id
                    ),
                });
                break; // one overlap issue per building is enough
            }
            seen_cells.insert(cell, b.instance_id);
        }
    }

    for b in &layout.buildings {
        let Some(def) = all().get(&b.building_identifier) else {
            continue;
        };
        let mut prereqs: Vec<&String> = def.prerequisites.iter().collect();
        prereqs.sort();
        for prereq in prereqs {
            if !prerequisite_satisfied(prereq, &identifiers) {
                issues.push(Issue {
                    kind: IssueKind::WithoutPrerequisite,
                    instance_id: b.instance_id,
                    detail: format!("requires {}", prereq.replace('|', " or ")),
                });
            }
        }
    }

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
    for (identifier, ids) in counts_list {
        let Some(def) = all().get(identifier) else {
            continue;
        };
        let max = def.max_allowed;
        // Sort deterministically by instance id; overflow instances are the tail.
        let mut ids = ids.clone();
        ids.sort_unstable();
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
        // Cheesemaker requires (Barn|GoatBarn) AND Tech_085. Only Barn exists in the dump,
        // so the (Barn|GoatBarn) prereq is satisfied while Tech_085 stays missing.
        add(&mut layout, "Barn", 0, 0);
        let id = add(&mut layout, "Cheesemaker", 60, 0);
        let issues = layout.validate();
        assert!(
            !issues
                .iter()
                .any(|i| i.instance_id == id && i.detail.contains("Barn"))
        );
        assert!(
            issues
                .iter()
                .any(|i| i.instance_id == id && i.detail.contains("Tech_085"))
        );
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
