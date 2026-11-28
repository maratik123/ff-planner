use std::hash::{Hash as _, Hasher as _};

use crate::buildings::{Category, all};
use crate::placement::Layout;
use crate::validation::Issue;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub total: u32,
    pub total_cells: u32,
    pub by_category: Vec<(Category, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct FrameCache {
    hash: Option<u64>,
    issues: Vec<Issue>,
    stats: Stats,
}

fn hash_layout(layout: &Layout) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    layout.next_instance_id.hash(&mut hasher);
    for b in &layout.buildings {
        b.instance_id.hash(&mut hasher);
        b.building_identifier.hash(&mut hasher);
        b.pos.hash(&mut hasher);
        b.rotation.hash(&mut hasher);
    }
    hasher.finish()
}

fn compute_stats(layout: &Layout) -> Stats {
    let mut by_category: std::collections::HashMap<Category, u32> =
        std::collections::HashMap::new();
    let mut total_cells = 0u32;
    let mut total = 0u32;
    for b in &layout.buildings {
        if let Some(def) = all().get(&b.building_identifier) {
            let dims = b.rotation.dimensions(def.grid);
            *by_category.entry(def.category).or_insert(0) += 1;
            total += 1;
            total_cells += dims.width * dims.height;
        }
    }
    let mut cat_list: Vec<(Category, u32)> = by_category.into_iter().collect();
    cat_list.sort_by_key(|(c, _)| *c);
    Stats {
        total,
        total_cells,
        by_category: cat_list,
    }
}

impl FrameCache {
    pub fn ensure(&mut self, layout: &Layout) {
        let hash = hash_layout(layout);
        if self.hash == Some(hash) {
            return;
        }
        self.hash = Some(hash);
        self.issues = layout.validate();
        self.stats = compute_stats(layout);
    }

    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::{Rotation, TilePos};

    fn layout_with_academy() -> Layout {
        let mut layout = Layout::default();
        layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("add valid");
        layout
    }

    #[test]
    fn stats_matches_layout_content() {
        let layout = layout_with_academy();
        let mut cache = FrameCache::default();
        cache.ensure(&layout);
        let stats = cache.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.total_cells, 20);
        assert_eq!(stats.by_category, vec![(Category::AmenitiesAndServices, 1)]);
    }

    #[test]
    fn issues_match_validate() {
        let layout = layout_with_academy();
        let mut cache = FrameCache::default();
        cache.ensure(&layout);
        assert_eq!(cache.issues(), layout.validate().as_slice());
    }

    #[test]
    fn cache_invalidates_on_layout_change() {
        let mut layout = layout_with_academy();
        let mut cache = FrameCache::default();
        cache.ensure(&layout);
        let before = cache.stats().clone();
        layout
            .add("Academy", TilePos { x: 10, y: 10 }, Rotation::Zero)
            .expect("add valid");
        cache.ensure(&layout);
        assert_eq!(cache.stats().total, 2);
        assert_ne!(cache.stats(), &before);
    }

    #[test]
    fn cache_is_reused_when_layout_unchanged() {
        let layout = layout_with_academy();
        let mut cache = FrameCache::default();
        cache.ensure(&layout);
        let issues_ptr = cache.issues().as_ptr();
        cache.ensure(&layout);
        assert_eq!(cache.issues().as_ptr(), issues_ptr);
    }
}
