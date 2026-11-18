//! Released actor tick world services.
use super::{
    actor::{Actor, Attack},
    add, down,
};

pub struct Settings {
    pub gravity: f32,
    pub height: f32,
    pub height_gain: f32,
}

pub trait World {
    fn find_cell(&mut self, position: [f32; 4]) -> Option<usize>;
    fn occluded(&mut self, start: [f32; 4], end: [f32; 4]) -> bool;
    /// Cast downward from the particle, excluding the actor's own hit cell.
    /// Return the signed intersection distance relative to the lowered origin.
    fn ground_distance(
        &mut self,
        position: [f32; 4],
        cell: Option<usize>,
        lowering: f32,
    ) -> Option<f32>;
}

impl Actor {
    pub fn tick<W: World>(
        &mut self,
        player: [f32; 4],
        settings: &Settings,
        world: &mut W,
    ) -> Option<Attack> {
        if self.initial.life <= 0 {
            return None;
        }
        for (value, word) in self
            .initial
            .matrix
            .iter_mut()
            .flatten()
            .zip(&self.initial.event_words[27..43])
        {
            *value = f32::from_bits(*word);
        }
        let (direction, distance) = self.player_direction(player);
        let radius = f32::from_bits(self.initial.copied.parameters[1]);
        let needs_visibility =
            (!self.active && distance < radius) || (self.active && distance > radius);
        let mut start = player;
        let mut end = self.body.particles[0].vectors[0];
        start[1] = down(add(f64::from(start[1]), 0.5));
        end[1] = down(add(f64::from(end[1]), 0.5));
        let blocked = needs_visibility && world.occluded(start, end);
        self.update_visibility(distance, || blocked, |position| world.find_cell(position));
        let mut attack = None;
        if self.active {
            self.advance_cooldowns();
            attack = self.select_attack(distance, player, |start, end| world.occluded(start, end));
            let mut displacement = self.steering_displacement(distance, direction);
            let lowering = down(add(f64::from(settings.gravity), f64::from(settings.height)));
            let clearance = match world.ground_distance(
                self.body.particles[0].vectors[0],
                self.body.particles[0].cell,
                lowering,
            ) {
                Some(distance) => down(add(f64::from(distance), f64::from(lowering))),
                None => lowering,
            };
            displacement[1] = Self::correct_height(
                displacement[1],
                clearance,
                settings.gravity,
                settings.height,
                settings.height_gain,
            );
            self.body.translate(displacement);
        }
        self.update_event_pose(direction);
        self.update_attack_modulation();
        self.update_health_modulation();
        attack
    }
}
