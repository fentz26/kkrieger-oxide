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
/// Projectile shot identities/event bindings are prepared before entering physics.
pub struct FrameInput<'a> {
    pub registry: &'a Registry,
    pub colliders: &'a [u32],
    pub scene: Vec<super::scene_cells::SceneCell>,
    pub dynamic_order: Option<&'a [super::registration::DynamicSlot]>,
    pub points: &'a [super::runtime::RegisteredPoint],
    pub boxes: &'a [super::runtime::RegisteredBox],
    pub physics_order: Option<&'a [super::projectile::PhysicsSlot]>,
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
        self.player_clearance_with_cells(position, cell, gravity, height, &[])
    }
    pub fn player_clearance_with_cells(
        &mut self,
        position: [f32; 4],
        cell: Option<usize>,
        gravity: f32,
        height: f32,
        scene: &[super::scene_cells::SceneCell],
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
        let mut views = scene
            .iter()
            .map(super::scene_cells::SceneCell::collision_view)
            .collect::<Result<Vec<_>, _>>()?;
        let mut runtime = CollisionRuntime {
            adds: &adds,
            candidates: CandidateWorld {
                static_cells: &statics,
                lists: &lists,
                dynamic_cells: &mut views,
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
        inventory: &mut crate::game::Inventory,
        damage: &[i32; 8],
        switches: &mut [u8; 256],
    ) -> Result<Vec<crate::game::player::Sound>, &'static str> {
        let adds = self.world.additive_views();
        accepted.apply_damage(registry, inventory, damage, switches, |point| {
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
        self.physics_with_scene(registry, colliders, prefix, settings, Vec::new())
    }
    pub fn physics_with_scene(
        &mut self,
        registry: &Registry,
        colliders: &[u32],
        prefix: &mut [(super::body::Body, super::runtime::BodyRegistration)],
        settings: &Settings,
        scene: Vec<super::scene_cells::SceneCell>,
    ) -> Result<ActorFrame, &'static str> {
        self.physics_with_order(registry, colliders, prefix, settings, scene, None)
    }
    pub fn physics_with_order(
        &mut self,
        registry: &Registry,
        colliders: &[u32],
        prefix: &mut [(super::body::Body, super::runtime::BodyRegistration)],
        settings: &Settings,
        scene: Vec<super::scene_cells::SceneCell>,
        order: Option<&[super::registration::DynamicSlot]>,
    ) -> Result<ActorFrame, &'static str> {
        self.physics_frame(
            FrameInput {
                registry,
                colliders,
                scene,
                dynamic_order: order,
                points: &[],
                boxes: &[],
                physics_order: None,
            },
            prefix,
            settings,
        )
    }
    pub fn physics_frame(
        &mut self,
        input: FrameInput<'_>,
        prefix: &mut [(super::body::Body, super::runtime::BodyRegistration)],
        settings: &Settings,
    ) -> Result<ActorFrame, &'static str> {
        let FrameInput {
            registry,
            colliders,
            scene,
            dynamic_order,
            points,
            boxes,
            physics_order,
        } = input;
        if self.zone_masks.len() != self.world.hulls.len() {
            return Err("released zone registry count differs");
        }
        let mut captured = ActorFrame::capture(registry, colliders, settings.first_particle)?;
        captured.attach_scene(scene)?;
        if let Some(order) = dynamic_order {
            captured.set_order(order)?;
        }
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
        let mut projectile_ids = std::collections::BTreeMap::new();
        for point in points {
            let pointer = (std::rc::Rc::as_ptr(&point.point) as usize, 0);
            if captured.particles.contains_key(&point.particle)
                || prefix_shots.contains_key(&point.particle)
                || projectile_ids
                    .insert(point.particle, pointer)
                    .is_some_and(|old| old != pointer)
            {
                return Err("released particle identity overlaps");
            }
        }
        for registration in boxes {
            let pointer = std::rc::Rc::as_ptr(&registration.projectile) as usize;
            for (index, id) in registration.particles.into_iter().enumerate() {
                let identity = (pointer, index);
                if captured.particles.contains_key(&id)
                    || prefix_shots.contains_key(&id)
                    || projectile_ids
                        .insert(id, identity)
                        .is_some_and(|old| old != identity)
                {
                    return Err("released particle identity overlaps");
                }
            }
        }
        let fallback_order: Vec<_> = if physics_order.is_none() {
            (0..colliders.len())
                .map(super::projectile::PhysicsSlot::Actor)
                .chain((0..points.len()).map(super::projectile::PhysicsSlot::Point))
                .chain((0..boxes.len()).map(super::projectile::PhysicsSlot::Box))
                .collect()
        } else {
            Vec::new()
        };
        let roster = super::runtime::PhysicsRoster {
            registry,
            colliders,
            registrations: &captured.bodies,
            points,
            boxes,
            order: physics_order.unwrap_or(&fallback_order),
        };
        let mut accepted = captured.clone();
        self.hits.retain_particles(
            |id| {
                captured.particles.contains_key(&id)
                    || prefix_shots.contains_key(&id)
                    || projectile_ids.contains_key(&id)
            },
            |id| projectile_ids.contains_key(&id),
        );
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
            runtime.step_mixed_roster(
                roster,
                prefix,
                settings.physics,
                |runtime, _, prefix, _| accepted.reconcile_all_with_roster(runtime, roster, prefix),
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
