use crate::placement::{Layout, PlacedBuilding, Rotation, TilePos};

/// A single undoable layout action with its inverse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Add {
        placed: PlacedBuilding,
    },
    Remove {
        placed: PlacedBuilding,
    },
    Move {
        instance_id: u64,
        from: TilePos,
        to: TilePos,
        from_rotation: Rotation,
        to_rotation: Rotation,
    },
    Rotate {
        instance_id: u64,
        from: Rotation,
        to: Rotation,
    },
    Batch(Vec<Self>),
}

impl Command {
    pub fn apply(&self, layout: &mut Layout) {
        match self {
            Self::Add { placed } => {
                layout.buildings.push(placed.clone());
                layout.next_instance_id = layout.next_instance_id.max(placed.instance_id + 1);
            }
            Self::Remove { placed } => {
                layout
                    .buildings
                    .retain(|b| b.instance_id != placed.instance_id);
            }
            Self::Move {
                instance_id,
                to,
                to_rotation,
                ..
            } => {
                if let Some(b) = layout
                    .buildings
                    .iter_mut()
                    .find(|b| b.instance_id == *instance_id)
                {
                    b.pos = *to;
                    b.rotation = *to_rotation;
                }
            }
            Self::Rotate {
                instance_id, to, ..
            } => {
                if let Some(b) = layout
                    .buildings
                    .iter_mut()
                    .find(|b| b.instance_id == *instance_id)
                {
                    b.rotation = *to;
                }
            }
            Self::Batch(cmds) => {
                for c in cmds {
                    c.apply(layout);
                }
            }
        }
    }

    pub fn revert(&self, layout: &mut Layout) {
        match self {
            Self::Add { placed } => {
                layout
                    .buildings
                    .retain(|b| b.instance_id != placed.instance_id);
            }
            Self::Remove { placed } => {
                layout.buildings.push(placed.clone());
                layout.next_instance_id = layout.next_instance_id.max(placed.instance_id + 1);
            }
            Self::Move {
                instance_id,
                from,
                from_rotation,
                ..
            } => {
                if let Some(b) = layout
                    .buildings
                    .iter_mut()
                    .find(|b| b.instance_id == *instance_id)
                {
                    b.pos = *from;
                    b.rotation = *from_rotation;
                }
            }
            Self::Rotate {
                instance_id, from, ..
            } => {
                if let Some(b) = layout
                    .buildings
                    .iter_mut()
                    .find(|b| b.instance_id == *instance_id)
                {
                    b.rotation = *from;
                }
            }
            Self::Batch(cmds) => {
                for c in cmds.iter().rev() {
                    c.revert(layout);
                }
            }
        }
    }
}

const MAX_HISTORY: usize = 100;

#[derive(Debug, Default)]
pub struct LayoutHistory {
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl LayoutHistory {
    pub fn exec(&mut self, cmd: Command, layout: &mut Layout) {
        cmd.apply(layout);
        self.undo.push(cmd);
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, layout: &mut Layout) -> bool {
        let Some(cmd) = self.undo.pop() else {
            return false;
        };
        cmd.revert(layout);
        self.redo.push(cmd);
        true
    }

    pub fn redo(&mut self, layout: &mut Layout) -> bool {
        let Some(cmd) = self.redo.pop() else {
            return false;
        };
        cmd.apply(layout);
        self.undo.push(cmd);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_add_undo_redo() {
        let mut layout = Layout::default();
        let mut h = LayoutHistory::default();
        let placed = PlacedBuilding {
            instance_id: 0,
            building_identifier: "Academy".to_owned(),
            pos: TilePos { x: 0, y: 0 },
            rotation: Rotation::Zero,
        };
        h.exec(Command::Add { placed }, &mut layout);
        assert_eq!(layout.buildings.len(), 1);
        assert!(h.undo(&mut layout));
        assert_eq!(layout.buildings.len(), 0);
        assert!(h.redo(&mut layout));
        assert_eq!(layout.buildings.len(), 1);
        assert_eq!(layout.buildings[0].instance_id, 0);
    }

    #[test]
    fn undo_remove_restores_building() {
        let mut layout = Layout::default();
        layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        let placed = layout.get(0).cloned().expect("unwrap");
        let mut h = LayoutHistory::default();
        h.exec(
            Command::Remove {
                placed: placed.clone(),
            },
            &mut layout,
        );
        assert_eq!(layout.buildings.len(), 0);
        assert!(h.undo(&mut layout));
        assert_eq!(layout.buildings.len(), 1);
        assert_eq!(layout.buildings[0], placed);
        assert!(h.redo(&mut layout));
        assert_eq!(layout.buildings.len(), 0);
    }

    #[test]
    fn rotate_undo_redo() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        let mut h = LayoutHistory::default();
        h.exec(
            Command::Rotate {
                instance_id: id,
                from: Rotation::Zero,
                to: Rotation::R90,
            },
            &mut layout,
        );
        assert_eq!(layout.get(id).expect("unwrap").rotation, Rotation::R90);
        assert!(h.undo(&mut layout));
        assert_eq!(layout.get(id).expect("unwrap").rotation, Rotation::Zero);
        assert!(h.redo(&mut layout));
        assert_eq!(layout.get(id).expect("unwrap").rotation, Rotation::R90);
    }

    #[test]
    fn move_undo_redo() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        let mut h = LayoutHistory::default();
        h.exec(
            Command::Move {
                instance_id: id,
                from: TilePos { x: 0, y: 0 },
                to: TilePos { x: 5, y: 5 },
                from_rotation: Rotation::Zero,
                to_rotation: Rotation::Zero,
            },
            &mut layout,
        );
        assert_eq!(layout.get(id).expect("unwrap").pos, TilePos { x: 5, y: 5 });
        assert!(h.undo(&mut layout));
        assert_eq!(layout.get(id).expect("unwrap").pos, TilePos { x: 0, y: 0 });
        assert!(h.redo(&mut layout));
        assert_eq!(layout.get(id).expect("unwrap").pos, TilePos { x: 5, y: 5 });
    }

    #[test]
    fn move_with_rotation_undo_redo() {
        let mut layout = Layout::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        let mut h = LayoutHistory::default();
        h.exec(
            Command::Move {
                instance_id: id,
                from: TilePos { x: 0, y: 0 },
                to: TilePos { x: 5, y: 5 },
                from_rotation: Rotation::Zero,
                to_rotation: Rotation::R90,
            },
            &mut layout,
        );
        let b = layout.get(id).expect("unwrap");
        assert_eq!(b.pos, TilePos { x: 5, y: 5 });
        assert_eq!(b.rotation, Rotation::R90);
        assert!(h.undo(&mut layout));
        let b = layout.get(id).expect("unwrap");
        assert_eq!(b.pos, TilePos { x: 0, y: 0 });
        assert_eq!(b.rotation, Rotation::Zero);
        assert!(h.redo(&mut layout));
        let b = layout.get(id).expect("unwrap");
        assert_eq!(b.pos, TilePos { x: 5, y: 5 });
        assert_eq!(b.rotation, Rotation::R90);
    }

    #[test]
    fn new_exec_clears_redo() {
        let mut layout = Layout::default();
        let mut h = LayoutHistory::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        h.exec(
            Command::Rotate {
                instance_id: id,
                from: Rotation::Zero,
                to: Rotation::R90,
            },
            &mut layout,
        );
        assert!(h.undo(&mut layout));
        assert!(h.can_redo());
        h.exec(
            Command::Rotate {
                instance_id: id,
                from: Rotation::Zero,
                to: Rotation::R180,
            },
            &mut layout,
        );
        assert!(!h.can_redo());
    }

    #[test]
    fn history_capped_at_100() {
        let mut layout = Layout::default();
        let mut h = LayoutHistory::default();
        let id = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("unwrap");
        for i in 0..150i32 {
            let from = if i % 2 == 0 {
                Rotation::Zero
            } else {
                Rotation::R180
            };
            let to = if i % 2 == 0 {
                Rotation::R180
            } else {
                Rotation::Zero
            };
            h.exec(
                Command::Rotate {
                    instance_id: id,
                    from,
                    to,
                },
                &mut layout,
            );
        }
        let mut count = 0;
        while h.undo(&mut layout) {
            count += 1;
        }
        assert_eq!(count, 100);
    }

    #[test]
    fn add_revert_keeps_next_id_ahead() {
        let mut layout = Layout::default();
        let mut h = LayoutHistory::default();
        let placed = PlacedBuilding {
            instance_id: 5,
            building_identifier: "Academy".to_owned(),
            pos: TilePos { x: 0, y: 0 },
            rotation: Rotation::Zero,
        };
        h.exec(Command::Add { placed }, &mut layout);
        assert!(h.undo(&mut layout));
        assert!(layout.next_instance_id >= 6);
    }

    #[test]
    fn batch_undo_redo() {
        let mut layout = Layout::default();
        let id_a = layout
            .add("Academy", TilePos { x: 0, y: 0 }, Rotation::Zero)
            .expect("a");
        let id_b = layout
            .add("Academy", TilePos { x: 10, y: 0 }, Rotation::Zero)
            .expect("b");
        let mut h = LayoutHistory::default();
        h.exec(
            Command::Batch(vec![
                Command::Rotate {
                    instance_id: id_a,
                    from: Rotation::Zero,
                    to: Rotation::R90,
                },
                Command::Rotate {
                    instance_id: id_b,
                    from: Rotation::Zero,
                    to: Rotation::R90,
                },
            ]),
            &mut layout,
        );
        assert_eq!(layout.get(id_a).expect("a").rotation, Rotation::R90);
        assert_eq!(layout.get(id_b).expect("b").rotation, Rotation::R90);
        assert!(h.undo(&mut layout));
        assert_eq!(layout.get(id_a).expect("a").rotation, Rotation::Zero);
        assert_eq!(layout.get(id_b).expect("b").rotation, Rotation::Zero);
        assert!(h.redo(&mut layout));
        assert_eq!(layout.get(id_a).expect("a").rotation, Rotation::R90);
        assert_eq!(layout.get(id_b).expect("b").rotation, Rotation::R90);
    }
}
