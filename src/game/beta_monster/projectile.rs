//! Released scene Physic point and box projectile state.
use super::body::Particle;
use crate::{mesh::Matrix, scene::event::Id};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsSlot {
    Actor(usize),
    Point(usize),
}
#[derive(Clone, Debug)]
pub struct Point {
    pub flags: u32,
    pub particle: Particle,
    pub event: Option<Id>,
    pub skip_player: bool,
}
impl Point {
    pub fn register_hit_state(
        &self,
        particle: usize,
        events: &crate::scene::event::Scheduler,
        hits: &mut super::hit::HitRegistry,
    ) -> Result<(), &'static str> {
        let words = self
            .event
            .map(|id| {
                events
                    .events
                    .get(&id)
                    .ok_or("released projectile event missing")?
                    .try_borrow()
                    .map_err(|_| "released projectile event already borrowed")
                    .map(|event| event.released_hit_state())
            })
            .transpose()?;
        if hits
            .bound_event(particle)
            .is_some_and(|id| Some(id) != self.event)
        {
            return Err("particle bound to another event");
        }
        hits.shots.insert(
            particle,
            super::hit::Shot {
                kind: self.particle.flags[3],
                event: words,
            },
        );
        if let Some((id, words)) = self.event.zip(words) {
            hits.bind_event(particle, id, words)?;
        }
        Ok(())
    }
    pub fn new(
        words: &[u32],
        matrix: Matrix,
        cell: Option<usize>,
        event: Option<Id>,
    ) -> Result<Self, &'static str> {
        let words = words
            .get(..12)
            .ok_or("truncated released Physic parameters")?;
        let flags = words[0];
        if flags & 3 != 0 {
            return Err("released point projectile requires Physic mode 0");
        }
        let speed = super::dynamic::rotate(
            std::array::from_fn(|axis| f32::from_bits(words[1 + axis])),
            matrix,
        );
        let mut particle = Particle::new(matrix[3], [speed[0], speed[1], speed[2], 0.], cell);
        particle.flags[0] = 1;
        particle.flags[3] = words[11];
        particle.weight = f32::from_bits(words[10]);
        let skip_player = flags & 0x20 != 0;
        Ok(Self {
            flags,
            particle,
            event: if skip_player { event } else { None },
            skip_player,
        })
    }
}

/// Validate the full event batch before applying hit fields to live scene events.
pub fn sync_hit_events(
    events: &crate::scene::event::Scheduler,
    hits: &super::hit::HitRegistry,
) -> Result<(), &'static str> {
    let updates = hits
        .event_states()
        .map(|(id, words)| {
            Ok((
                events
                    .events
                    .get(&id)
                    .cloned()
                    .ok_or("released projectile event missing")?,
                words,
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let mut borrowed = updates
        .iter()
        .map(|(event, _)| {
            event
                .try_borrow_mut()
                .map_err(|_| "released projectile event already borrowed")
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (event, (_, words)) in borrowed.iter_mut().zip(&updates) {
        event.apply_released_hit_state(words);
    }
    Ok(())
}

/// Prepare one frame's point identities and event snapshots before physics.
/// Repeated registrations retain the same particle identity and roster position.
pub fn prepare_points(
    points: &[std::rc::Rc<std::cell::RefCell<Point>>],
    first_particle: usize,
    player_cell: Option<usize>,
    events: &crate::scene::event::Scheduler,
    hits: &mut super::hit::HitRegistry,
) -> Result<Vec<super::runtime::RegisteredPoint>, &'static str> {
    let mut identities = std::collections::BTreeMap::new();
    let mut staged = super::hit::HitRegistry::default();
    let mut registrations = Vec::with_capacity(points.len());
    for point in points {
        let pointer = std::rc::Rc::as_ptr(point);
        let particle = if let Some(&id) = identities.get(&pointer) {
            id
        } else {
            let id = first_particle
                .checked_add(identities.len())
                .ok_or("released point particle identity overflow")?;
            identities.insert(pointer, id);
            id
        };
        // Physics needs exclusive access; detect outstanding shared borrows here too.
        let state = point
            .try_borrow_mut()
            .map_err(|_| "released point already borrowed")?;
        state.register_hit_state(particle, events, &mut staged)?;
        registrations.push(super::runtime::RegisteredPoint {
            point: std::rc::Rc::clone(point),
            particle,
            skip_cell: if state.skip_player { player_cell } else { None },
        });
    }
    hits.replace_projectile_bindings(staged);
    Ok(registrations)
}

/// Persistent released Physic mode-1 state, shared by frame registrations.
#[derive(Clone, Debug)]
pub struct BoxProjectile {
    pub flags: u32,
    pub body: super::body::BoxBody,
    pub event: Option<Id>,
    pub skip_player: bool,
}
impl BoxProjectile {
    pub fn new(
        words: &[u32],
        matrix: Matrix,
        cell: Option<usize>,
        event: Option<Id>,
    ) -> Result<Self, &'static str> {
        let words = words
            .get(..12)
            .ok_or("truncated released Physic parameters")?;
        let flags = words[0];
        if flags & 3 != 1 {
            return Err("released box projectile requires Physic mode 1");
        }
        let speed = super::dynamic::rotate(
            std::array::from_fn(|axis| f32::from_bits(words[1 + axis])),
            matrix,
        );
        let scale = std::array::from_fn(|axis| f32::from_bits(words[4 + axis]));
        let mut body =
            super::body::BoxBody::new(matrix, scale, [speed[0], speed[1], speed[2], 0.], cell);
        for particle in &mut body.particles {
            particle.flags[3] = words[11];
            particle.weight = f32::from_bits(words[10]);
        }
        let skip_player = flags & 0x20 != 0;
        Ok(Self {
            flags,
            body,
            event: if skip_player { event } else { None },
            skip_player,
        })
    }
}
