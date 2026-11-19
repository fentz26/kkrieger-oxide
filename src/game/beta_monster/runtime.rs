//! Borrowed registry adapter for the released collision response.
use super::{
    body::Particle,
    collision::{self, Cast, Completion, Contact, ResponseWorld},
    hit::HitRegistry,
    subtractive::{CandidateQuery, CandidateWorld, DynamicEffects, ParticleContact, ParticleInfo},
    world_cast::{AdditiveCell, Ray, cast_world},
    zones::{self, Transition, Zone},
};

pub struct CollisionRuntime<'a, 'g> {
    pub adds: &'a [AdditiveCell<'g>],
    pub candidates: CandidateWorld<'a, 'g>,
    pub zones: &'a mut [Zone<'g>],
    pub zone_lists: &'a [&'g [usize]],
    pub hits: &'a mut HitRegistry,
    pub particle_id: usize,
    pub skip_cell: Option<usize>,
    pub monster_mode: bool,
    pub interrupted: bool,
    pub error: Option<&'static str>,
}
impl CollisionRuntime<'_, '_> {
    fn fail(&mut self, reason: &'static str) {
        self.error.get_or_insert(reason);
        self.interrupted = true;
    }
    pub fn collide(
        &mut self,
        particle: &mut Particle,
        mode: bool,
    ) -> Result<Completion, &'static str> {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        let identity = self.hits.first_particle(self.particle_id)?;
        if identity.kind != particle.flags[3] {
            return Err("particle kind differs from registry");
        }
        if particle.cell.is_none() {
            return Err("collision particle has no world cell");
        }
        if self
            .skip_cell
            .is_some_and(|id| id >= self.candidates.dynamic_cells.len())
        {
            return Err("skip cell out of range");
        }
        let result = collision::respond(particle, mode, self);
        if let Some(error) = self.error.or(self.hits.error) {
            Err(error)
        } else {
            Ok(result)
        }
    }
}
impl ResponseWorld for CollisionRuntime<'_, '_> {
    fn skip_cell(&mut self, _: &Particle, skipped: bool) {
        if let Some(id) = self.skip_cell {
            if skipped {
                self.candidates.dynamic_cells[id].mode |= 0x10;
            } else {
                self.candidates.dynamic_cells[id].mode &= !0x10;
            }
        }
    }
    fn clear_interrupt(&mut self) {
        self.interrupted = false;
    }
    fn interrupted(&self) -> bool {
        self.interrupted || self.error.is_some() || self.hits.error.is_some()
    }
    fn cast(&mut self, particle: &mut Particle, cast: &mut Cast, first: bool) -> Option<Contact> {
        if self.error.is_some() {
            return None;
        }
        let Some(mut current) = cast.cell else {
            self.fail("cast has no world cell");
            return None;
        };
        let first = if first {
            match self.hits.first_particle(self.particle_id) {
                Ok(value) => Some(value),
                Err(error) => {
                    self.fail(error);
                    return None;
                }
            }
        } else {
            None
        };
        let mut query = CandidateQuery {
            ray: Ray {
                direction: cast.direction,
                end: cast.target,
                start: cast.origin,
            },
            particle: Some(ParticleInfo {
                kind: particle.flags[3],
                mass: particle.weight,
            }),
            first,
            monster_mode: self.monster_mode,
        };
        let result = cast_world(
            self.adds,
            &mut current,
            &mut self.candidates,
            &mut query,
            &mut self.interrupted,
            self.hits,
        );
        cast.cell = Some(current);
        match result {
            Ok(value) => value.map(|hit| Contact {
                plane: hit.plane,
                distance: hit.distance,
            }),
            Err(error) => {
                self.fail(error);
                None
            }
        }
    }
    fn shot(&mut self, particle: &mut Particle, impact: &mut [f32; 4], plane: &mut [f32; 4]) {
        let first = match self.hits.first_particle(self.particle_id) {
            Ok(value) => value,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        if first.shot_select.is_none() {
            return;
        }
        let mut select = first.shot_select;
        self.hits.particle_hit(
            ParticleContact {
                cell: usize::MAX,
                particle: self.particle_id,
                kind: particle.flags[3],
                position: *impact,
                plane: *plane,
                monster: false,
            },
            &mut select,
        );
    }
    fn zones(
        &mut self,
        particle: &mut Particle,
        target: &mut [f32; 4],
        old: Option<usize>,
        new: Option<usize>,
    ) {
        if self.error.is_some() {
            return;
        }
        let (Some(old_cell), Some(new_cell)) = (old, new) else {
            self.fail("zone transition has no world cell");
            return;
        };
        let shot_select = match self.hits.first_particle(self.particle_id) {
            Ok(value) => value.shot_select,
            Err(error) => {
                self.fail(error);
                return;
            }
        };
        if let Err(error) = zones::transition(
            self.zones,
            self.zone_lists,
            Transition {
                old_cell,
                new_cell,
                start: particle.vectors[1],
                end: *target,
                kind: particle.flags[3],
                shot_select,
            },
        ) {
            self.fail(error)
        }
    }
}

impl CollisionRuntime<'_, '_> {
    /// Check identity and owned-cell metadata before particle mutation.
    fn validate_particle(&self, particle: &Particle) -> Result<(), &'static str> {
        self.validate_particle_at(particle, self.particle_id, self.skip_cell)
    }
    fn validate_particle_at(
        &self,
        particle: &Particle,
        id: usize,
        skip: Option<usize>,
    ) -> Result<(), &'static str> {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        let identity = self.hits.first_particle(id)?;
        if identity.kind != particle.flags[3] {
            return Err("particle kind differs from registry");
        }
        if skip.is_some_and(|id| id >= self.candidates.dynamic_cells.len()) {
            return Err("skip cell out of range");
        }
        Ok(())
    }
    pub fn step_single(
        &mut self,
        particle: &mut Particle,
        settings: super::physics::Settings,
    ) -> Result<(), &'static str> {
        self.validate_particle(particle)?;
        particle.step(settings, self)?;
        if let Some(error) = self.error.or(self.hits.error) {
            Err(error)
        } else {
            Ok(())
        }
    }
    pub fn step_particle(
        &mut self,
        body: &mut super::body::Body,
        index: usize,
        settings: super::physics::Settings,
    ) -> Result<(), &'static str> {
        let particle = body
            .particles
            .get(index)
            .ok_or("particle index out of range")?;
        self.validate_particle(particle)?;
        body.step_particle(index, settings, self)?;
        if let Some(error) = self.error.or(self.hits.error) {
            Err(error)
        } else {
            Ok(())
        }
    }
}
impl super::physics::World for CollisionRuntime<'_, '_> {
    fn outside(&mut self, cell: usize, point: [f32; 4]) -> bool {
        match self.adds.get(cell) {
            Some(cell) => super::hull::outside_mask(cell.planes, point, 0.) != 0,
            None => {
                self.fail("particle world cell out of range");
                true
            }
        }
    }
    fn find_cell(&mut self, point: [f32; 4]) -> Option<usize> {
        if self.error.is_some() {
            return None;
        }
        super::hull::find_cell(
            self.adds
                .iter()
                .enumerate()
                .map(|(id, cell)| (id, cell.planes)),
            point,
        )
    }
    fn collide(&mut self, particle: &mut Particle) -> bool {
        match CollisionRuntime::collide(self, particle, false) {
            Ok(completion) => completion != Completion::Clear,
            Err(error) => {
                self.fail(error);
                false
            }
        }
    }
}

/// Stable identities and the owned collision cell for one registered body.
#[derive(Clone, Copy, Debug)]
pub struct BodyRegistration {
    pub particles: [usize; 5],
    pub skip_cell: Option<usize>,
}
#[derive(Clone)]
pub struct RegisteredPoint {
    pub point: std::rc::Rc<std::cell::RefCell<super::projectile::Point>>,
    pub particle: usize,
    pub skip_cell: Option<usize>,
}
#[derive(Clone, Copy)]
pub struct PhysicsRoster<'a> {
    pub registry: &'a super::registry::Registry,
    pub colliders: &'a [u32],
    pub registrations: &'a std::collections::BTreeMap<u32, BodyRegistration>,
    pub points: &'a [RegisteredPoint],
    pub order: &'a [super::projectile::PhysicsSlot],
}
impl CollisionRuntime<'_, '_> {
    fn validate_body(
        &self,
        body: &super::body::Body,
        registration: &BodyRegistration,
    ) -> Result<(), &'static str> {
        for (particle, id) in body.particles.iter().zip(registration.particles) {
            if self.hits.first_particle(id)?.kind != particle.flags[3] {
                return Err("particle kind differs from registry");
            }
        }
        if registration
            .skip_cell
            .is_some_and(|id| id >= self.candidates.dynamic_cells.len())
        {
            return Err("skip cell out of range");
        }
        if body
            .particles
            .iter()
            .enumerate()
            .any(|(i, p)| i > 0 && matches!(p.flags[3], 1 | 3))
        {
            return Err("five-particle group truncated");
        }
        if body
            .constraints
            .iter()
            .any(|c| c.particles.iter().any(|&i| i >= 5))
        {
            return Err("constraint particle index out of range");
        }
        Ok(())
    }
    pub fn step_body<F>(
        &mut self,
        body: &mut super::body::Body,
        ids: [usize; 5],
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        F: FnOnce(&mut Self, &mut super::body::Body) -> Result<(), &'static str>,
    {
        let registration = BodyRegistration {
            particles: ids,
            skip_cell: self.skip_cell,
        };
        self.step_bodies(
            std::slice::from_mut(body),
            &[registration],
            settings,
            |runtime, bodies| dynamic(runtime, &mut bodies[0]),
        )
    }
    /// Global released phase order for registered bodies. Dynamic geometry is
    /// processed once after all particles, before any pending forces are applied.
    pub fn step_bodies<B, F>(
        &mut self,
        bodies: &mut [B],
        registrations: &[BodyRegistration],
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        B: AsRef<super::body::Body> + AsMut<super::body::Body>,
        F: FnOnce(&mut Self, &mut [B]) -> Result<(), &'static str>,
    {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        if bodies.len() != registrations.len() {
            return Err("body registration count differs");
        }
        for (body, registration) in bodies.iter().zip(registrations) {
            let body = body.as_ref();
            self.validate_body(body, registration)?;
        }
        let previous = (self.particle_id, self.skip_cell);
        let result = (|| {
            for body in bodies.iter_mut() {
                let body = body.as_mut();
                for i in 0..body.constraints.len() {
                    body.solve_constraint(i)?;
                }
            }
            for (body, registration) in bodies.iter_mut().zip(registrations) {
                let body = body.as_mut();
                self.skip_cell = registration.skip_cell;
                for (index, id) in registration.particles.into_iter().enumerate() {
                    self.particle_id = id;
                    self.step_particle(body, index, settings)?;
                }
            }
            dynamic(self, bodies)?;
            if let Some(error) = self.error.or(self.hits.error) {
                return Err(error);
            }
            for body in bodies {
                body.as_mut().apply_forces();
            }
            Ok(())
        })();
        self.particle_id = previous.0;
        self.skip_cell = previous.1;
        result
    }
}

impl CollisionRuntime<'_, '_> {
    /// Reconcile one moving cell against registered bodies using real mode-1
    /// collision probes. Publish its accepted collision view after this returns.
    pub fn reconcile_cell<B: AsRef<super::body::Body> + AsMut<super::body::Body>>(
        &mut self,
        index: usize,
        cell: &mut super::dynamic::MovingCell,
        bodies: &mut [B],
        registrations: &[BodyRegistration],
    ) -> Result<(), &'static str> {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        if index >= self.candidates.dynamic_cells.len() {
            return Err("moving cell index out of range");
        }
        if bodies.len() != registrations.len() {
            return Err("body registration count differs");
        }
        for (body, registration) in bodies.iter().zip(registrations) {
            let body = body.as_ref();
            if registration
                .skip_cell
                .is_some_and(|i| i >= self.candidates.dynamic_cells.len())
            {
                return Err("skip cell out of range");
            }
            for (p, id) in body.particles.iter().zip(registration.particles) {
                if self.hits.first_particle(id)?.kind != p.flags[3] {
                    return Err("particle kind differs from registry");
                }
            }
        }
        let previous = (self.particle_id, self.skip_cell);
        let particles = bodies
            .iter_mut()
            .zip(registrations)
            .flat_map(|(body, registration)| {
                body.as_mut()
                    .particles
                    .iter_mut()
                    .zip(registration.particles)
                    .map(move |(particle, id)| {
                        (
                            particle,
                            registration.skip_cell == Some(index),
                            (id, registration.skip_cell),
                        )
                    })
            });
        let result = cell.reconcile_with(particles, |(id, skip), particle| {
            self.particle_id = id;
            self.skip_cell = skip;
            self.collide(particle, true)
        });
        self.particle_id = previous.0;
        self.skip_cell = previous.1;
        result
    }
}

/// AI queries share the collision registry and its error state.
pub struct AiWorld<'r, 'a, 'g> {
    pub runtime: &'r mut CollisionRuntime<'a, 'g>,
    pub player_cell: Option<usize>,
}
impl AiWorld<'_, '_, '_> {
    fn ray(&mut self, mut cell: usize, ray: Ray) -> Option<crate::collision::Intersection> {
        if self.runtime.error.is_some() {
            return None;
        }
        let mut query = CandidateQuery {
            ray,
            particle: None,
            first: None,
            monster_mode: self.runtime.monster_mode,
        };
        match cast_world(
            self.runtime.adds,
            &mut cell,
            &mut self.runtime.candidates,
            &mut query,
            &mut self.runtime.interrupted,
            self.runtime.hits,
        ) {
            Ok(hit) => hit,
            Err(error) => {
                self.runtime.fail(error);
                None
            }
        }
    }
}
impl super::ai::World for AiWorld<'_, '_, '_> {
    fn find_cell(&mut self, position: [f32; 4]) -> Option<usize> {
        super::hull::find_cell(
            self.runtime
                .adds
                .iter()
                .enumerate()
                .map(|(id, cell)| (id, cell.planes)),
            position,
        )
    }
    fn occluded(&mut self, start: [f32; 4], end: [f32; 4]) -> bool {
        let Some(cell) = self.player_cell else {
            self.runtime.fail("AI visibility has no player cell");
            return true;
        };
        let direction = std::array::from_fn(|axis| {
            super::down(super::add(f64::from(end[axis]), -f64::from(start[axis])))
        });
        self.ray(
            cell,
            Ray {
                direction,
                start,
                end,
            },
        )
        .is_some()
    }
    fn ground_distance(
        &mut self,
        position: [f32; 4],
        cell: Option<usize>,
        lowering: f32,
    ) -> Option<f32> {
        let Some(cell) = cell else {
            self.runtime.fail("AI ground cast has no world cell");
            return None;
        };
        let skip = self.runtime.skip_cell;
        if let Some(id) = skip {
            let Some(view) = self.runtime.candidates.dynamic_cells.get_mut(id) else {
                self.runtime.fail("AI skip cell out of range");
                return None;
            };
            view.mode |= 0x10;
        }
        let mut end = position;
        end[1] = super::down(super::add(f64::from(end[1]), -f64::from(lowering)));
        let result = self.ray(
            cell,
            Ray {
                direction: [0., -1., 0.],
                start: position,
                end,
            },
        );
        if let Some(id) = skip {
            self.runtime.candidates.dynamic_cells[id].mode &= !0x10;
        }
        result.map(|hit| hit.distance)
    }
}

impl CollisionRuntime<'_, '_> {
    /// Run an actor against this collision registry and dispatch its attack.
    pub fn tick_actor(
        &mut self,
        actor: &mut super::actor::Actor,
        player: [f32; 4],
        player_cell: Option<usize>,
        settings: &super::ai::Settings,
        inventory: &mut crate::game::Inventory,
    ) -> Result<Option<crate::game::player::Sound>, &'static str> {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        let attack = actor.tick(
            player,
            settings,
            &mut AiWorld {
                runtime: self,
                player_cell,
            },
        );
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        match attack {
            Some(super::actor::Attack::Melee(hits)) => Ok(inventory.hit(hits)),
            Some(super::actor::Attack::Ranged(event)) => {
                self.hits.pending.push(event);
                Ok(None)
            }
            None => Ok(None),
        }
    }
}

impl CollisionRuntime<'_, '_> {
    /// Tick actors in registration order, selecting each owned dynamic cell.
    pub fn tick_actors(
        &mut self,
        actors: &mut [(super::actor::Actor, Option<usize>)],
        player: [f32; 4],
        player_cell: Option<usize>,
        settings: &super::ai::Settings,
        inventory: &mut crate::game::Inventory,
    ) -> Result<Vec<crate::game::player::Sound>, &'static str> {
        if actors
            .iter()
            .any(|(_, cell)| cell.is_some_and(|id| id >= self.candidates.dynamic_cells.len()))
        {
            return Err("AI owned cell out of range");
        }
        let previous = self.skip_cell;
        let mut sounds = Vec::new();
        let result = (|| {
            for (actor, cell) in actors {
                self.skip_cell = *cell;
                if let Some(sound) =
                    self.tick_actor(actor, player, player_cell, settings, inventory)?
                {
                    sounds.push(sound);
                }
            }
            Ok(sounds)
        })();
        self.skip_cell = previous;
        result
    }
}

impl CollisionRuntime<'_, '_> {
    pub fn step_registered<F>(
        &mut self,
        registry: &super::registry::Registry,
        registrations: &std::collections::BTreeMap<u32, BodyRegistration>,
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        F: FnOnce(&mut Self, &super::registry::Registry) -> Result<(), &'static str>,
    {
        let mut tokens = Vec::new();
        registry.visit_active(|token, actor| {
            if actor.active {
                tokens.push(token);
            }
            Ok(())
        })?;
        self.step_collider_roster(registry, &tokens, registrations, settings, dynamic)
    }
    pub fn step_collider_roster<F>(
        &mut self,
        registry: &super::registry::Registry,
        colliders: &[u32],
        registrations: &std::collections::BTreeMap<u32, BodyRegistration>,
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        F: FnOnce(&mut Self, &super::registry::Registry) -> Result<(), &'static str>,
    {
        self.step_body_roster(
            registry,
            colliders,
            registrations,
            &mut [],
            settings,
            |runtime, registry, _| dynamic(runtime, registry),
        )
    }
    pub fn step_body_roster<F>(
        &mut self,
        registry: &super::registry::Registry,
        colliders: &[u32],
        registrations: &std::collections::BTreeMap<u32, BodyRegistration>,
        prefix: &mut [(super::body::Body, BodyRegistration)],
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        F: FnOnce(
            &mut Self,
            &super::registry::Registry,
            &mut [(super::body::Body, BodyRegistration)],
        ) -> Result<(), &'static str>,
    {
        let order: Vec<_> = (0..colliders.len())
            .map(super::projectile::PhysicsSlot::Actor)
            .collect();
        self.step_mixed_roster(
            PhysicsRoster {
                registry,
                colliders,
                registrations,
                points: &[],
                order: &order,
            },
            prefix,
            settings,
            |runtime, registry, prefix, _| dynamic(runtime, registry, prefix),
        )
    }
    /// Player prefix bodies precede the mixed actor/point registration sequence.
    /// Dynamic processing runs after every particle step and before every force add.
    fn validate_roster(
        &self,
        roster: PhysicsRoster<'_>,
        prefix: &[(super::body::Body, BodyRegistration)],
    ) -> Result<(), &'static str> {
        use super::projectile::PhysicsSlot;
        let PhysicsRoster {
            registry,
            colliders,
            registrations,
            points,
            order,
        } = roster;
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        if order.len() != colliders.len() + points.len() {
            return Err("released physics order differs from registry");
        }
        let mut seen_actors = vec![false; colliders.len()];
        let mut seen_points = vec![false; points.len()];
        for slot in order {
            let seen = match *slot {
                PhysicsSlot::Actor(index) => seen_actors.get_mut(index),
                PhysicsSlot::Point(index) => seen_points.get_mut(index),
            }
            .ok_or("released physics order differs from registry")?;
            if std::mem::replace(seen, true) {
                return Err("released physics order differs from registry");
            }
        }
        for (body, registration) in prefix.iter() {
            self.validate_body(body, registration)?;
        }
        registry.visit_tokens(colliders, |token, actor| {
            self.validate_body(
                &actor.body,
                registrations
                    .get(&token)
                    .ok_or("released body registration missing")?,
            )
        })?;
        for registration in points {
            let point = registration
                .point
                .try_borrow_mut()
                .map_err(|_| "released point already borrowed")?;
            self.validate_particle_at(
                &point.particle,
                registration.particle,
                registration.skip_cell,
            )?;
            if matches!(point.particle.flags[3], 1 | 3) {
                return Err("five-particle group truncated");
            }
        }
        Ok(())
    }
    pub fn step_mixed_roster<F>(
        &mut self,
        roster: PhysicsRoster<'_>,
        prefix: &mut [(super::body::Body, BodyRegistration)],
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        F: FnOnce(
            &mut Self,
            &super::registry::Registry,
            &mut [(super::body::Body, BodyRegistration)],
            &[RegisteredPoint],
        ) -> Result<(), &'static str>,
    {
        use super::projectile::PhysicsSlot;
        self.validate_roster(roster, prefix)?;
        let PhysicsRoster {
            registry,
            colliders,
            registrations,
            points,
            order,
        } = roster;
        let previous = (self.particle_id, self.skip_cell);
        let result = (|| {
            for (body, _) in prefix.iter_mut() {
                for index in 0..body.constraints.len() {
                    body.solve_constraint(index)?;
                }
            }
            for slot in order {
                if let PhysicsSlot::Actor(index) = *slot {
                    registry.visit_tokens(&colliders[index..index + 1], |_, actor| {
                        for index in 0..actor.body.constraints.len() {
                            actor.body.solve_constraint(index)?;
                        }
                        Ok(())
                    })?;
                }
            }
            for (body, registration) in prefix.iter_mut() {
                self.skip_cell = registration.skip_cell;
                for (index, id) in registration.particles.into_iter().enumerate() {
                    self.particle_id = id;
                    self.step_particle(body, index, settings)?;
                }
            }
            for slot in order {
                match *slot {
                    PhysicsSlot::Actor(index) => {
                        registry.visit_tokens(&colliders[index..index + 1], |token, actor| {
                            let registration = &registrations[&token];
                            self.skip_cell = registration.skip_cell;
                            for (index, id) in registration.particles.into_iter().enumerate() {
                                self.particle_id = id;
                                self.step_particle(&mut actor.body, index, settings)?;
                            }
                            Ok(())
                        })?
                    }
                    PhysicsSlot::Point(index) => {
                        let registration = &points[index];
                        let mut point = registration
                            .point
                            .try_borrow_mut()
                            .map_err(|_| "released point already borrowed")?;
                        self.particle_id = registration.particle;
                        self.skip_cell = registration.skip_cell;
                        self.step_single(&mut point.particle, settings)?;
                    }
                }
            }
            dynamic(self, registry, prefix, points)?;
            if let Some(error) = self.error.or(self.hits.error) {
                return Err(error);
            }
            for (body, _) in prefix.iter_mut() {
                body.apply_forces();
            }
            for slot in order {
                match *slot {
                    PhysicsSlot::Actor(index) => {
                        registry.visit_tokens(&colliders[index..index + 1], |_, actor| {
                            actor.body.apply_forces();
                            Ok(())
                        })?
                    }
                    PhysicsSlot::Point(index) => points[index]
                        .point
                        .try_borrow_mut()
                        .map_err(|_| "released point already borrowed")?
                        .particle
                        .apply_force(),
                }
            }
            Ok(())
        })();
        self.particle_id = previous.0;
        self.skip_cell = previous.1;
        result
    }
}

impl CollisionRuntime<'_, '_> {
    pub fn tick_registered(
        &mut self,
        registry: &super::registry::Registry,
        registrations: &std::collections::BTreeMap<u32, BodyRegistration>,
        player: [f32; 4],
        player_cell: Option<usize>,
        settings: &super::ai::Settings,
        inventory: &mut crate::game::Inventory,
    ) -> Result<Vec<crate::game::player::Sound>, &'static str> {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        registry.visit_active(|token, actor| {
            let registration = registrations
                .get(&token)
                .ok_or("released body registration missing")?;
            self.validate_body(&actor.body, registration)
        })?;
        let previous = self.skip_cell;
        let mut sounds = Vec::new();
        let result = registry.visit_active(|token, actor| {
            self.skip_cell = registrations[&token].skip_cell;
            if let Some(sound) = self.tick_actor(actor, player, player_cell, settings, inventory)? {
                sounds.push(sound);
            }
            Ok(())
        });
        self.skip_cell = previous;
        result.map(|()| sounds)
    }
}

struct BorrowedBody<'a>(&'a mut super::body::Body);
impl AsRef<super::body::Body> for BorrowedBody<'_> {
    fn as_ref(&self) -> &super::body::Body {
        self.0
    }
}
impl AsMut<super::body::Body> for BorrowedBody<'_> {
    fn as_mut(&mut self) -> &mut super::body::Body {
        self.0
    }
}
impl CollisionRuntime<'_, '_> {
    /// Reconcile against the same mixed body order used by the particle phase.
    pub fn reconcile_mixed_roster(
        &mut self,
        index: usize,
        cell: &mut super::dynamic::MovingCell,
        roster: PhysicsRoster<'_>,
        prefix: &mut [(super::body::Body, BodyRegistration)],
    ) -> Result<(), &'static str> {
        use super::projectile::PhysicsSlot;
        if index >= self.candidates.dynamic_cells.len() {
            return Err("moving cell index out of range");
        }
        self.validate_roster(roster, prefix)?;
        let previous = (self.particle_id, self.skip_cell);
        let result = cell.reconcile_visit(
            |visit| {
                for (body, registration) in prefix.iter_mut() {
                    for (particle, id) in body.particles.iter_mut().zip(registration.particles) {
                        visit(
                            particle,
                            registration.skip_cell == Some(index),
                            (id, registration.skip_cell),
                        )?;
                    }
                }
                for slot in roster.order {
                    match *slot {
                        PhysicsSlot::Actor(actor_index) => roster.registry.visit_tokens(
                            &roster.colliders[actor_index..actor_index + 1],
                            |token, actor| {
                                let registration = &roster.registrations[&token];
                                for (particle, id) in
                                    actor.body.particles.iter_mut().zip(registration.particles)
                                {
                                    visit(
                                        particle,
                                        registration.skip_cell == Some(index),
                                        (id, registration.skip_cell),
                                    )?;
                                }
                                Ok(())
                            },
                        )?,
                        PhysicsSlot::Point(point_index) => {
                            let registration = &roster.points[point_index];
                            let mut point = registration
                                .point
                                .try_borrow_mut()
                                .map_err(|_| "released point already borrowed")?;
                            visit(
                                &mut point.particle,
                                registration.skip_cell == Some(index),
                                (registration.particle, registration.skip_cell),
                            )?;
                        }
                    }
                }
                Ok(())
            },
            |(id, skip), particle| {
                self.particle_id = id;
                self.skip_cell = skip;
                self.collide(particle, true)
            },
        );
        self.particle_id = previous.0;
        self.skip_cell = previous.1;
        result
    }
    pub fn reconcile_registered(
        &mut self,
        index: usize,
        cell: &mut super::dynamic::MovingCell,
        registry: &super::registry::Registry,
        registrations: &std::collections::BTreeMap<u32, BodyRegistration>,
    ) -> Result<(), &'static str> {
        self.reconcile_body_roster(index, cell, registry, registrations, &mut [])
    }
    pub fn reconcile_body_roster(
        &mut self,
        index: usize,
        cell: &mut super::dynamic::MovingCell,
        registry: &super::registry::Registry,
        registrations: &std::collections::BTreeMap<u32, BodyRegistration>,
        prefix: &mut [(super::body::Body, BodyRegistration)],
    ) -> Result<(), &'static str> {
        let mut seen = std::collections::BTreeSet::new();
        let mut handles = Vec::new();
        let mut registration_list: Vec<_> = prefix
            .iter()
            .map(|(_, registration)| *registration)
            .collect();
        for &token in registry.active_ids() {
            if !seen.insert(token) {
                continue;
            }
            let actor = registry
                .get(token)
                .ok_or("released actor identity missing")?;
            let active = actor
                .try_borrow()
                .map_err(|_| "released actor already borrowed")?
                .active;
            if !active {
                continue;
            }
            registration_list.push(
                *registrations
                    .get(&token)
                    .ok_or("released body registration missing")?,
            );
            handles.push(actor);
        }
        let mut actors = handles
            .iter()
            .map(|actor| {
                actor
                    .try_borrow_mut()
                    .map_err(|_| "released actor already borrowed")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut bodies: Vec<_> = prefix
            .iter_mut()
            .map(|(body, _)| BorrowedBody(body))
            .chain(actors.iter_mut().map(|actor| BorrowedBody(&mut actor.body)))
            .collect();
        self.reconcile_cell(index, cell, &mut bodies, &registration_list)
    }
}

impl CollisionRuntime<'_, '_> {
    /// Released box phase order: constraints, particles, dynamics, then forces.
    pub fn step_box<F>(
        &mut self,
        body: &mut super::body::BoxBody,
        ids: [usize; 8],
        settings: super::physics::Settings,
        dynamic: F,
    ) -> Result<(), &'static str>
    where
        F: FnOnce(&mut Self, &mut super::body::BoxBody) -> Result<(), &'static str>,
    {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        for (particle, id) in body.particles.iter().zip(ids) {
            self.validate_particle_at(particle, id, self.skip_cell)?;
            if matches!(particle.flags[3], 1 | 3) {
                return Err("five-particle group truncated");
            }
        }
        if body
            .constraints
            .iter()
            .any(|c| c.particles.iter().any(|&i| i >= 8))
        {
            return Err("constraint particle index out of range");
        }
        let previous = (self.particle_id, self.skip_cell);
        let result = (|| {
            for index in 0..body.constraints.len() {
                body.solve_constraint(index)?;
            }
            for (particle, id) in body.particles.iter_mut().zip(ids) {
                self.particle_id = id;
                self.step_single(particle, settings)?;
            }
            dynamic(self, body)?;
            if let Some(error) = self.error.or(self.hits.error) {
                return Err(error);
            }
            for particle in &mut body.particles {
                particle.apply_force();
            }
            Ok(())
        })();
        self.particle_id = previous.0;
        self.skip_cell = previous.1;
        result
    }
}
