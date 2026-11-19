//! Frame collision geometry separated from mutable persistent actors.
use super::{
    dynamic::{CollisionProperties, MovingCell},
    hit::Shot,
    registry::Registry,
    runtime::BodyRegistration,
    subtractive::DynamicCell,
};
use std::collections::BTreeMap;
#[derive(Clone)]
pub struct ActorFrame {
    pub cells: Vec<(u32, MovingCell)>,
    collided: Vec<u32>,
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
        let collided = vec![0; cells.len()];
        Ok(Self {
            collided,
            cells,
            bodies,
            particles,
        })
    }
    pub fn save_collision_masks(&mut self, masks: &[u32]) -> Result<(), &'static str> {
        if masks.len() != self.cells.len() {
            return Err("released collision mask count differs");
        }
        self.collided.copy_from_slice(masks);
        Ok(())
    }
    /// Commit accepted geometry only after all actor identities can be borrowed.
    pub fn publish_geometry(&self, registry: &Registry) -> Result<(), &'static str> {
        let mut latest = BTreeMap::new();
        for (index, (token, _)) in self.cells.iter().enumerate() {
            latest.insert(*token, index);
        }
        let handles = latest
            .keys()
            .map(|&token| {
                registry
                    .get(token)
                    .ok_or("released collider identity missing")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut actors = handles
            .iter()
            .map(|handle| {
                handle
                    .try_borrow_mut()
                    .map_err(|_| "released actor already borrowed")
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (actor, index) in actors.iter_mut().zip(latest.values()) {
            actor.hit_cell = self.cells[*index].1.clone();
        }
        Ok(())
    }

    pub fn dynamic_views(&self) -> Result<Vec<DynamicCell<'_>>, &'static str> {
        self.cells
            .iter()
            .enumerate()
            .map(|(index, (token, cell))| {
                cell.collision_view(CollisionProperties {
                    only_for_shot: false,
                    player: false,
                    part_box: Some(*token as usize),
                    collided: self.collided[index],
                })
            })
            .collect()
    }
}

impl ActorFrame {
    /// Rebuild borrowed cast views after each accepted geometry update.
    pub fn reconcile_all(
        &mut self,
        runtime: &mut super::runtime::CollisionRuntime<'_, '_>,
        registry: &Registry,
    ) -> Result<(), &'static str> {
        if runtime.candidates.dynamic_cells.len() != self.cells.len() {
            return Err("released dynamic registry count differs");
        }
        self.collided = runtime
            .candidates
            .dynamic_cells
            .iter()
            .map(|cell| cell.collided)
            .collect();
        for index in 0..self.cells.len() {
            let mut proposed = self.cells[index].1.clone();
            let masks = {
                let mut views = self.dynamic_views()?;
                let mut zones = runtime.zones.to_vec();
                let mut probe = super::runtime::CollisionRuntime {
                    adds: runtime.adds,
                    candidates: super::subtractive::CandidateWorld {
                        static_cells: runtime.candidates.static_cells,
                        lists: runtime.candidates.lists,
                        dynamic_cells: &mut views,
                    },
                    zones: &mut zones,
                    zone_lists: runtime.zone_lists,
                    hits: runtime.hits,
                    particle_id: runtime.particle_id,
                    skip_cell: runtime.skip_cell,
                    monster_mode: runtime.monster_mode,
                    interrupted: runtime.interrupted,
                    error: runtime.error,
                };
                let result =
                    probe.reconcile_registered(index, &mut proposed, registry, &self.bodies);
                runtime.interrupted = probe.interrupted;
                runtime.error = probe.error;
                for (target, source) in runtime.zones.iter_mut().zip(probe.zones.iter()) {
                    target.enter = source.enter;
                    target.leave = source.leave;
                    target.collided = source.collided;
                }
                result?;
                probe
                    .candidates
                    .dynamic_cells
                    .iter()
                    .map(|cell| cell.collided)
                    .collect()
            };
            self.collided = masks;
            self.cells[index].1 = proposed;
        }
        self.publish_geometry(registry)
    }
}
