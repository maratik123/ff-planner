use std::collections::HashSet;

use crate::history::Command;
use crate::placement::Layout;

/// Toggle `id` in `selected`. Returns true if the id is now selected.
pub(crate) fn toggle_selection(selected: &mut Vec<u64>, id: u64) -> bool {
    if let Some(pos) = selected.iter().position(|&s| s == id) {
        selected.remove(pos);
        false
    } else {
        selected.push(id);
        true
    }
}

/// Extend `selected` with the layout-order range between the first selected id
/// (the anchor) and `id`, inclusive. Used by Shift+click.
pub(crate) fn extend_selection_range(layout: &Layout, selected: &mut Vec<u64>, id: u64) {
    let Some(anchor) = selected.first().copied() else {
        if !selected.contains(&id) {
            selected.push(id);
        }
        return;
    };
    let a = layout
        .buildings
        .iter()
        .position(|b| b.instance_id == anchor);
    let b = layout.buildings.iter().position(|b| b.instance_id == id);
    let (Some(a), Some(b)) = (a, b) else {
        if !selected.contains(&id) {
            selected.push(id);
        }
        return;
    };
    let (lo, hi) = (a.min(b), a.max(b));
    for building in &layout.buildings[lo..=hi] {
        if !selected.contains(&building.instance_id) {
            selected.push(building.instance_id);
        }
    }
}

/// Drop ids that no longer exist in the layout and remove duplicates.
pub(crate) fn prune_selection(layout: &Layout, selected: &mut Vec<u64>) {
    let alive: HashSet<u64> = layout.buildings.iter().map(|b| b.instance_id).collect();
    let mut seen = HashSet::new();
    selected.retain(|id| alive.contains(id) && seen.insert(*id));
}

/// Build a single Batch command deleting every id present in the layout.
pub(crate) fn batch_delete(layout: &Layout, ids: &[u64]) -> Option<Command> {
    let mut cmds = Vec::new();
    for id in ids {
        if let Some(b) = layout.get(*id) {
            cmds.push(Command::Remove { placed: b.clone() });
        }
    }
    if cmds.is_empty() {
        None
    } else {
        Some(Command::Batch(cmds))
    }
}

/// Build a single Batch command rotating every selected building 90°.
/// Returns None if anything fails (missing building or any placement would
/// overlap another building or another selected building).
pub(crate) fn batch_rotate(layout: &Layout, ids: &[u64]) -> Option<Command> {
    let ignore: HashSet<u64> = ids.iter().copied().collect();
    let cache = crate::placement::OccupiedCells::new(layout);
    let mut cmds = Vec::new();
    for id in ids {
        let b = layout.get(*id)?;
        let new = b.rotation.next();
        if !cache.can_place_ignoring(&b.building_identifier, b.pos, new, &ignore) {
            return None;
        }
        cmds.push(Command::Rotate {
            instance_id: *id,
            from: b.rotation,
            to: new,
        });
    }
    if cmds.is_empty() {
        None
    } else {
        Some(Command::Batch(cmds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::{Layout, Rotation, TilePos};

    fn add_two() -> (Layout, u64, u64) {
        let mut layout = Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("a");
        let b = layout
            .add("Academy", TilePos { x: 10, y: 0 }, Rotation::Zero)
            .expect("b");
        (layout, a, b)
    }

    #[test]
    fn toggle_adds_and_removes() {
        let mut sel = Vec::new();
        assert!(toggle_selection(&mut sel, 1));
        assert!(sel.contains(&1));
        assert!(!toggle_selection(&mut sel, 1));
        assert!(sel.is_empty(), "selection should be empty");
    }

    #[test]
    fn toggle_ignores_nothing_but_toggles() {
        let mut sel = vec![1];
        assert!(toggle_selection(&mut sel, 2));
        assert_eq!(sel, vec![1, 2]);
    }

    #[test]
    fn extend_range_selects_between_anchor_and_id() {
        let (layout, a, b) = add_two();
        let mut sel = vec![a];
        extend_selection_range(&layout, &mut sel, b);
        assert!(sel.contains(&a));
        assert!(sel.contains(&b));
    }

    #[test]
    fn extend_range_selects_non_adjacent_range() {
        let mut layout = Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("a");
        let b = layout
            .add("Academy", TilePos { x: 10, y: 0 }, Rotation::Zero)
            .expect("b");
        let c = layout
            .add("Academy", TilePos { x: 20, y: 0 }, Rotation::Zero)
            .expect("c");
        let mut sel = vec![a];
        extend_selection_range(&layout, &mut sel, c);
        assert!(sel.contains(&a));
        assert!(sel.contains(&b));
        assert!(sel.contains(&c));
    }
    #[test]
    fn prune_removes_stale_and_duplicates() {
        let (layout, a, _) = add_two();
        let mut sel = vec![a, a, 999];
        prune_selection(&layout, &mut sel);
        assert_eq!(sel, vec![a]);
    }

    #[test]
    fn batch_delete_removes_each() {
        let (layout, a, b) = add_two();
        let cmd = batch_delete(&layout, &[a, b]).expect("batch");
        match cmd {
            Command::Batch(cmds) => assert_eq!(cmds.len(), 2),
            _ => panic!("expected Batch"),
        }
    }

    #[test]
    fn batch_delete_empty_when_no_ids() {
        let layout = Layout::default();
        assert!(batch_delete(&layout, &[]).is_none());
    }

    #[test]
    fn batch_rotate_valid_when_positions_free() {
        let mut layout = Layout::default();
        let a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("a");
        let b = layout
            .add("Academy", TilePos { x: 10, y: 10 }, Rotation::Zero)
            .expect("b");
        let cmd = batch_rotate(&layout, &[a, b]).expect("batch");
        let mut l = layout.clone();
        cmd.apply(&mut l);
        assert_eq!(l.get(a).expect("a").rotation, Rotation::R90);
        assert_eq!(l.get(b).expect("b").rotation, Rotation::R90);
    }

    fn manual_layout_with_overlap() -> (Layout, u64) {
        let mut layout = Layout::default();
        layout.buildings.push(crate::placement::PlacedBuilding {
            instance_id: 0,
            building_identifier: "Academy".to_owned(),
            pos: TilePos { x: 0, y: 0 },
            rotation: Rotation::Zero,
        });
        layout.buildings.push(crate::placement::PlacedBuilding {
            instance_id: 1,
            building_identifier: "Academy".to_owned(),
            pos: TilePos { x: 2, y: 0 },
            rotation: Rotation::Zero,
        });
        layout.next_instance_id = 2;
        (layout, 0)
    }

    #[test]
    fn batch_rotate_none_when_rotated_would_overlap() {
        let (layout, id) = manual_layout_with_overlap();
        // Rotated id=0 spans x 0..3, y 0..5 — overlaps id=1 (x 2..7, y 0..4).
        assert!(batch_rotate(&layout, &[id]).is_none());
    }

    #[test]
    fn batch_rotate_none_for_missing_id() {
        let (layout, a, _) = add_two();
        assert!(batch_rotate(&layout, &[a, 999]).is_none());
    }

    #[test]
    fn batch_delete_ignores_missing_ids() {
        let (layout, a, _) = add_two();
        let cmd = batch_delete(&layout, &[a, 999]).expect("batch");
        match cmd {
            Command::Batch(cmds) => assert_eq!(cmds.len(), 1),
            _ => panic!("expected Batch"),
        }
    }
}
