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
    /// Advance one particle through released integration and real collision.
    /// Caller selects the registered particle identity before each invocation.
    pub fn step_particle(
        &mut self,
        body: &mut super::body::Body,
        index: usize,
        settings: super::physics::Settings,
    ) -> Result<(), &'static str> {
        if let Some(error) = self.error.or(self.hits.error) {
            return Err(error);
        }
        let particle = body
            .particles
            .get(index)
            .ok_or("particle index out of range")?;
        let identity = self.hits.first_particle(self.particle_id)?;
        if identity.kind != particle.flags[3] {
            return Err("particle kind differs from registry");
        }
        if self
            .skip_cell
            .is_some_and(|id| id >= self.candidates.dynamic_cells.len())
        {
            return Err("skip cell out of range");
        }
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
