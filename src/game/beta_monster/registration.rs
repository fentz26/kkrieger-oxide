//! Frame collision geometry separated from mutable persistent actors.
use super::{
    dynamic::{CollisionProperties, MovingCell},
    hit::Shot,
    registry::Registry,
    runtime::BodyRegistration,
    subtractive::DynamicCell,
};
use std::collections::BTreeMap;
pub struct ActorFrame {
    pub cells: Vec<(u32, MovingCell)>,
    pub bodies: BTreeMap<u32, BodyRegistration>,
    pub particles: BTreeMap<usize, Shot>,
}
impl ActorFrame {
    pub fn capture(
        registry: &Registry,
        colliders: &[u32],
        first_particle: usize,
    ) -> Result<Self, &'static str> {
        let mut cells = Vec::with_capacity(colliders.len());
        for &token in colliders {
            let actor = registry
                .get(token)
                .ok_or("released collider identity missing")?;
            let actor = actor
                .try_borrow()
                .map_err(|_| "released actor already borrowed")?;
            cells.push((token, actor.hit_cell.clone()));
        }
        let mut bodies = BTreeMap::new();
        let mut particles = BTreeMap::new();
        registry.visit_active(|token, actor| {
            if bodies.contains_key(&token) {
                return Ok(());
            }
            let base = (token as usize - 1)
                .checked_mul(5)
                .and_then(|n| first_particle.checked_add(n))
                .ok_or("released particle identity exhausted")?;
            base.checked_add(4)
                .ok_or("released particle identity exhausted")?;
            let ids = std::array::from_fn(|i| base + i);
            for (particle, id) in actor.body.particles.iter().zip(ids) {
                particles.insert(
                    id,
                    Shot {
                        kind: particle.flags[3],
                        event: None,
                    },
                );
            }
            bodies.insert(
                token,
                BodyRegistration {
                    particles: ids,
                    skip_cell: cells.iter().position(|(id, _)| *id == token),
                },
            );
            Ok(())
        })?;
        Ok(Self {
            cells,
            bodies,
            particles,
        })
    }
    pub fn dynamic_views(&self) -> Result<Vec<DynamicCell<'_>>, &'static str> {
        self.cells
            .iter()
            .map(|(token, cell)| {
                cell.collision_view(CollisionProperties {
                    only_for_shot: false,
                    player: false,
                    part_box: Some(*token as usize),
                    collided: 0,
                })
            })
            .collect()
    }
}
