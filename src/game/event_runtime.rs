//! Original game AddEvents and persistent projectile/event ownership.
use super::{
    player::{Motion, Sound},
    shot, zones,
};
use crate::{
    collision::world::World,
    random::Random,
    scene::{
        event::{Event, Id},
        exec::Execution,
    },
};
pub struct Shot {
    pub state: shot::Spawn,
    pub event: Id,
}
pub struct Runtime {
    /// World entry identity and its two persistent zone events, in zone order.
    pub zones: Vec<(usize, [Option<Id>; 2])>,
    pub weapon: Id,
    pub shots: Vec<Shot>,
}
impl Runtime {
    pub fn restart(
        &self,
        execution: &mut Execution,
        motion: &mut Motion,
        world: &World,
        zones: &mut zones::Runtime,
    ) -> Result<(), &'static str> {
        let resets = execution
            .game
            .restart(motion, world, &mut execution.switches, zones)?;
        self.reset_zone_events(execution, &resets)
    }

    pub fn new(world: &World, execution: &mut Execution) -> Self {
        let zones = world
            .zones
            .iter()
            .map(|&entry| {
                let events = std::array::from_fn(|slot| {
                    world.entries[entry].events[slot].as_ref().map(|source| {
                        execution.events.insert(
                            Event {
                                operator: Some(source.operator),
                                translate: source.translate,
                                cull_distance: source.cull_distance as i32,
                                ..Event::default()
                            },
                            false,
                        )
                    })
                });
                (entry, events)
            })
            .collect();
        let weapon = execution.events.insert(Event::default(), false);
        Self {
            zones,
            weapon,
            shots: Vec::new(),
        }
    }
    /// AddEvents runs after InitFrame registration and before scene execution.
    pub fn add_events(
        &mut self,
        execution: &mut Execution,
        motion: &Motion,
        zones: &zones::Runtime,
    ) -> Result<(), &'static str> {
        if self.zones.len() != zones.respawn.len() {
            return Err("event zones do not match respawn state");
        }
        let mut i = 0;
        while i < self.shots.len() {
            let shot = &self.shots[i];
            let event = execution
                .events
                .events
                .get_mut(&shot.event)
                .ok_or("shot event missing")?;
            let selected = event.borrow().select != 0;
            if selected {
                // Source removes the shot record without calling Shot::Exit in this path.
                self.shots.swap_remove(i);
            } else {
                if shot.state.mode != 0 {
                    event.borrow_mut().matrix = shot.state.matrix;
                    execution.events.add(shot.event);
                }
                i += 1;
            }
        }
        for ((_, events), &respawn) in self.zones.iter().zip(&zones.respawn) {
            if respawn == 0
                && let Some(id) = events[0]
            {
                execution.events.add(id);
            }
            if respawn > 0
                && let Some(id) = events[1]
            {
                execution.events.add(id);
            }
        }
        if execution.game.weapon.next >= 8 || execution.game.weapon.current >= 8 {
            return Err("weapon identity out of range");
        }
        if execution.game.weapon.finish_switch() {
            execution.reset_event_instances(self.weapon);
            *execution
                .events
                .events
                .get(&self.weapon)
                .ok_or("weapon event missing")?
                .borrow_mut() = Event {
                operator: execution.game.events.optics[execution.game.weapon.current],
                ..Event::default()
            };
        }
        let event = execution
            .events
            .events
            .get_mut(&self.weapon)
            .ok_or("weapon event missing")?;
        let mut event = event.borrow_mut();
        if event.operator.is_some() {
            event.translate = motion.collider.position;
            event.translate[1] = motion.last_camera_y + execution.game.parameters.camera_offset;
            event.rotate = [
                motion.look / std::f32::consts::TAU,
                motion.direction / std::f32::consts::TAU,
                0.,
                0.,
            ];
            event.velocity = execution.game.weapon.timer;
            event.modulation = 0.;
            let cooldown = execution.game.parameters.weapon_cool[execution.game.weapon.current];
            if cooldown > 0.01 {
                event.modulation = (cooldown - execution.game.weapon.cooldown) / cooldown;
            }
            drop(event);
            execution.events.add(self.weapon);
        }
        Ok(())
    }
    /// Consume ordered DoRespawn requests from the zone phase.
    pub fn reset_zone_events(
        &self,
        execution: &mut Execution,
        entries: &[usize],
    ) -> Result<(), &'static str> {
        for entry in entries {
            let (_, events) = self
                .zones
                .iter()
                .find(|(id, _)| id == entry)
                .ok_or("zone event identity missing")?;
            if let Some(id) = events[1] {
                execution.reset_event_instances(id);
            }
        }
        Ok(())
    }
    pub fn fire(
        &mut self,
        execution: &mut Execution,
        weapon: usize,
        origin: shot::Origin,
        rng: &mut Random,
    ) -> Result<(), &'static str> {
        let operator = *execution
            .game
            .events
            .shot
            .get(weapon)
            .ok_or("shot weapon out of range")?;
        if let Some(state) = shot::spawn(weapon, operator, origin, rng)? {
            let event = execution.events.insert(
                Event {
                    operator: state.event_operator,
                    matrix: state.event_matrix,
                    monster: state.event_monster,
                    ..Event::default()
                },
                false,
            );
            self.shots.push(Shot { state, event });
        }
        Ok(())
    }
    /// Original ShotAI loop: damage, explosion creation, then shot Exit and swap
    /// removal. Newly created explosions become active on the next InitFrame.
    pub fn advance_shots(
        &mut self,
        execution: &mut Execution,
        motion: &Motion,
        world: &mut World,
    ) -> Result<Vec<Sound>, &'static str> {
        let mut sounds = Vec::new();
        let mut i = 0;
        while i < self.shots.len() {
            let outcome = self.shots[i].state.advance(
                world,
                &execution.dynamic_cells,
                Some(&motion.hit_cell),
                &execution.game.parameters,
                &execution.game.events,
            )?;
            let effects = execution
                .game
                .apply_shot_outcome(outcome, &mut execution.switches)?;
            if let Some(sound) = effects.sound {
                sounds.push(sound);
            }
            if let Some(explosion) = effects.explosion {
                execution.events.insert(
                    Event {
                        operator: explosion.operator,
                        matrix: explosion.matrix,
                        ..Event::default()
                    },
                    true,
                );
            }
            if effects.alive {
                i += 1;
            } else {
                let shot = self.shots.swap_remove(i);
                execution.reset_event_instances(shot.event);
                execution.events.events.remove(&shot.event);
            }
        }
        Ok(sounds)
    }
}
