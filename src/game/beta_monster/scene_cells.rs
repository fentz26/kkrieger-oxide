//! Bridge scene-owned dynamic instances to released hull and pose arithmetic.
use super::{
    dynamic::{MovingCell, Pose},
    hull::Hull,
};
use crate::collision::{DynamicCell, Interactions};
use std::{cell::RefCell, rc::Rc};

pub struct SceneCell {
    pub source: Rc<RefCell<DynamicCell>>,
    pub cell: MovingCell,
    pub only_for_shot: bool,
    pub player: bool,
    pub actor: Option<u32>,
    pub interactions: Interactions,
}
impl SceneCell {
    pub fn collision_view(&self) -> Result<super::subtractive::DynamicCell<'_>, &'static str> {
        self.cell
            .collision_view(super::dynamic::CollisionProperties {
                only_for_shot: self.only_for_shot,
                player: self.player,
                part_box: None,
                collided: self.interactions.collided,
            })
    }
    pub fn capture(source: Rc<RefCell<DynamicCell>>, player: bool) -> Result<Self, &'static str> {
        let (cell, only_for_shot, actor, interactions) = {
            let state = source
                .try_borrow()
                .map_err(|_| "scene dynamic cell already borrowed")?;
            let mode = if player {
                (state.cell.mode & !3) | 1
            } else {
                state.cell.mode
            };
            let hull = Hull::from_vertices(state.original_vertices, mode);
            if hull.count > 6 {
                return Err("dynamic plane capacity exceeded");
            }
            let mut original = [[0.; 4]; 6];
            original[..hull.count].copy_from_slice(&hull.planes[..hull.count]);
            let mut pose = Pose::apply(&original[..hull.count], state.current_matrix)?;
            pose.blocked = state.blocked != 0;
            let actor = state
                .monster
                .map(u32::try_from)
                .transpose()
                .map_err(|_| "released actor identity out of range")?;
            (
                MovingCell {
                    original,
                    count: hull.count,
                    confirmed: state.confirmed_matrix,
                    current: state.current_matrix,
                    mode,
                    monster: actor.is_some(),
                    pose,
                },
                state.cell.only_for_shot,
                actor,
                state.cell.interactions,
            )
        };
        Ok(Self {
            source,
            cell,
            only_for_shot,
            player,
            actor,
            interactions,
        })
    }
    /// Publish collision planes, accepted transforms and logic masks to the same instance.
    pub fn publish(&self) -> Result<(), &'static str> {
        let original = self
            .cell
            .original
            .get(..self.cell.count)
            .ok_or("dynamic plane capacity exceeded")?;
        let planes = self
            .cell
            .pose
            .planes
            .get(..self.cell.pose.count)
            .ok_or("dynamic plane capacity exceeded")?;
        let mut state = self
            .source
            .try_borrow_mut()
            .map_err(|_| "scene dynamic cell already borrowed")?;
        state.original_planes = original.to_vec();
        state.cell.planes = planes.to_vec();
        state.cell.center = self.cell.pose.center;
        state.cell.mode = self.cell.mode;
        state.cell.only_for_shot = self.only_for_shot;
        state.cell.interactions = self.interactions;
        state.current_matrix = self.cell.pose.matrix;
        state.confirmed_matrix = self.cell.confirmed;
        state.blocked = i32::from(self.cell.pose.blocked);
        Ok(())
    }
}
