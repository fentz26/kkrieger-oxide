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
        if self.zone_masks.len() != self.world.hulls.len() {
            return Err("released zone registry count differs");
        }
        let captured = ActorFrame::capture(registry, colliders, settings.first_particle)?;
        let mut accepted = captured.clone();
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
            runtime.step_collider_roster(
                registry,
                colliders,
                &captured.bodies,
                settings.physics,
                |runtime, registry| accepted.reconcile_all(runtime, registry),
            )?;
        }
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
