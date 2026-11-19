//! Owned static-world and actor frame-stage composition.
use super::{
    ai, hit::HitRegistry, physics, registration::ActorFrame, registry::Registry,
    runtime::CollisionRuntime, static_world::StaticWorld, subtractive::CandidateWorld, zones::Zone,
};

pub struct Settings {
    pub physics: physics::Settings,
    pub ai: ai::Settings,
    pub player: [f32; 4],
    pub player_cell: Option<usize>,
    pub first_particle: usize,
}
pub struct ActorSimulation {
    pub world: StaticWorld,
    pub hits: HitRegistry,
    pub zone_masks: Vec<[u32; 3]>,
}
impl ActorSimulation {
    /// Released player ground ray has no particle, so actor hulls are excluded.
    pub fn player_clearance(
        &mut self,
        position: [f32; 4],
        cell: Option<usize>,
        gravity: f32,
        height: f32,
    ) -> Result<f32, &'static str> {
        use super::ai::World;
        let base = super::down(super::add(f64::from(gravity), f64::from(height)));
        if cell.is_none() {
            return Ok(base);
        }
        let adds = self.world.additive_views();
        let statics = self.world.static_views();
        let lists: Vec<_> = self
            .world
            .subtractive_lists
            .iter()
            .map(Vec::as_slice)
            .collect();
        let mut runtime = CollisionRuntime {
            adds: &adds,
            candidates: CandidateWorld {
                static_cells: &statics,
                lists: &lists,
                dynamic_cells: &mut [],
            },
            zones: &mut [],
            zone_lists: &[],
            hits: &mut self.hits,
            particle_id: 0,
            skip_cell: None,
            monster_mode: false,
            interrupted: false,
            error: None,
        };
        let distance = super::runtime::AiWorld {
            runtime: &mut runtime,
            player_cell: cell,
        }
        .ground_distance(position, cell, base);
        if let Some(error) = runtime.error.or(runtime.hits.error) {
            return Err(error);
        }
        Ok(distance.map_or(base, |distance| {
            super::down(super::add(f64::from(base), f64::from(distance)))
        }))
    }
    pub fn apply_weapon_hits(
        &self,
        accepted: &mut ActorFrame,
        registry: &Registry,
        damage: &[i32; 8],
        switches: &mut [u8; 256],
    ) -> Result<(), &'static str> {
        let adds = self.world.additive_views();
        accepted.apply_weapon_hits(registry, damage, switches, |point| {
            super::hull::find_cell(
                adds.iter().enumerate().map(|(id, cell)| (id, cell.planes)),
                point,
            )
        })
    }
    /// Transfer source-indexed collision masks before the shared zone logic runs.
    pub fn publish_zone_interactions(
        &mut self,
        world: &mut crate::collision::world::World,
    ) -> Result<(), &'static str> {
        if self.zone_masks.len() != self.world.hulls.len()
            || self.zone_masks.len() != world.entries.len()
        {
            return Err("released zone registry count differs");
        }
        for (masks, entry) in self.zone_masks.iter_mut().zip(&mut world.entries) {
            entry.cell.interactions.enter |= masks[0];
            entry.cell.interactions.leave |= masks[1];
            entry.cell.interactions.collided |= masks[2];
            *masks = [0; 3];
        }
        Ok(())
    }
    pub fn new(world: StaticWorld) -> Self {
        let zone_masks = vec![[0; 3]; world.hulls.len()];
        Self {
            world,
            hits: HitRegistry::default(),
            zone_masks,
        }
    }
    pub fn step(
        &mut self,
        registry: &Registry,
        colliders: &[u32],
        inventory: &mut crate::game::Inventory,
        settings: &Settings,
    ) -> Result<Vec<crate::game::player::Sound>, &'static str> {
        let accepted = self.physics(registry, colliders, settings)?;
        self.think(registry, &accepted, inventory, settings)
    }
    pub fn physics(
        &mut self,
        registry: &Registry,
        colliders: &[u32],
        settings: &Settings,
    ) -> Result<ActorFrame, &'static str> {
        self.physics_with_bodies(registry, colliders, &mut [], settings)
    }
    pub fn physics_with_bodies(
        &mut self,
        registry: &Registry,
        colliders: &[u32],
        prefix: &mut [(super::body::Body, super::runtime::BodyRegistration)],
        settings: &Settings,
    ) -> Result<ActorFrame, &'static str> {
        if self.zone_masks.len() != self.world.hulls.len() {
            return Err("released zone registry count differs");
        }
        let captured = ActorFrame::capture(registry, colliders, settings.first_particle)?;
        let mut prefix_shots = std::collections::BTreeMap::new();
        for (body, registration) in prefix.iter() {
            for (particle, id) in body.particles.iter().zip(registration.particles) {
                if captured.particles.contains_key(&id) || prefix_shots.contains_key(&id) {
                    return Err("released particle identity overlaps");
                }
                prefix_shots.insert(
                    id,
                    super::hit::Shot {
                        kind: particle.flags[3],
                        event: None,
                    },
                );
            }
        }
        let mut accepted = captured.clone();
        self.hits.shots.extend(prefix_shots);
        self.hits.shots.extend(captured.particles.clone());
        let adds = self.world.additive_views();
        let statics = self.world.static_views();
        let lists: Vec<_> = self
            .world
            .subtractive_lists
            .iter()
            .map(Vec::as_slice)
            .collect();
        let zone_lists: Vec<_> = self.world.zone_lists.iter().map(Vec::as_slice).collect();
        let mut zones: Vec<_> = self
            .world
            .hulls
            .iter()
            .zip(&self.zone_masks)
            .map(|(hull, masks)| Zone {
                planes: &hull.planes[..hull.count],
                enter: masks[0],
                leave: masks[1],
                collided: masks[2],
            })
            .collect();
        {
            let mut views = captured.dynamic_views()?;
            let mut runtime = CollisionRuntime {
                adds: &adds,
                candidates: CandidateWorld {
                    static_cells: &statics,
                    lists: &lists,
                    dynamic_cells: &mut views,
                },
                zones: &mut zones,
                zone_lists: &zone_lists,
                hits: &mut self.hits,
                particle_id: settings.first_particle,
                skip_cell: None,
                monster_mode: false,
                interrupted: false,
                error: None,
            };
            runtime.step_body_roster(
                registry,
                colliders,
                &captured.bodies,
                prefix,
                settings.physics,
                |runtime, registry, prefix| {
                    accepted.reconcile_all_with_bodies(runtime, registry, prefix)
                },
            )?;
        }
        for (target, zone) in self.zone_masks.iter_mut().zip(zones) {
            *target = [zone.enter, zone.leave, zone.collided];
        }
        Ok(accepted)
    }
    pub fn think(
        &mut self,
        registry: &Registry,
        accepted: &ActorFrame,
        inventory: &mut crate::game::Inventory,
        settings: &Settings,
    ) -> Result<Vec<crate::game::player::Sound>, &'static str> {
        if self.zone_masks.len() != self.world.hulls.len() {
            return Err("released zone registry count differs");
        }
        let adds = self.world.additive_views();
        let statics = self.world.static_views();
        let lists: Vec<_> = self
            .world
            .subtractive_lists
            .iter()
            .map(Vec::as_slice)
            .collect();
        let zone_lists: Vec<_> = self.world.zone_lists.iter().map(Vec::as_slice).collect();
        let mut zones: Vec<_> = self
            .world
            .hulls
            .iter()
            .zip(&self.zone_masks)
            .map(|(hull, masks)| Zone {
                planes: &hull.planes[..hull.count],
                enter: masks[0],
                leave: masks[1],
                collided: masks[2],
            })
            .collect();
        let sounds = {
            let mut views = accepted.dynamic_views()?;
            let mut runtime = CollisionRuntime {
                adds: &adds,
                candidates: CandidateWorld {
                    static_cells: &statics,
                    lists: &lists,
                    dynamic_cells: &mut views,
                },
                zones: &mut zones,
                zone_lists: &zone_lists,
                hits: &mut self.hits,
                particle_id: settings.first_particle,
                skip_cell: None,
                monster_mode: false,
                interrupted: false,
                error: None,
            };
            runtime.tick_registered(
                registry,
                &accepted.bodies,
                settings.player,
                settings.player_cell,
                &settings.ai,
                inventory,
            )?
        };
        for (target, zone) in self.zone_masks.iter_mut().zip(zones) {
            *target = [zone.enter, zone.leave, zone.collided];
        }
        Ok(sounds)
    }
}
