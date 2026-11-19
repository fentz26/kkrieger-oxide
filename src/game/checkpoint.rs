//! Original respawn registration and Continue placement order.
use super::{
    State,
    player::{Motion, Sound},
};
use crate::collision::world::World;

#[derive(Clone, Copy, Debug, Default)]
pub struct Checkpoint {
    pub valid: bool,
    pub active: i32,
    pub position: [f32; 4],
    pub direction: f32,
}
impl State {
    /// Original Restart: operator reset, resolved camera position, all monster
    /// states and zone instance chains, then gameplay switches (options survive).
    pub fn restart(
        &mut self,
        motion: &mut Motion,
        world: &World,
        switches: &mut [u8; 256],
        zones: &mut super::zones::Runtime,
    ) -> Result<Vec<usize>, &'static str> {
        if zones.respawn.len() != world.zones.len() {
            return Err("zone runtime does not match world");
        }
        self.reset_by_operator = true;
        motion.place(world, self.parameters.start_position, 0., 0.)?;
        motion.last_camera_y = motion.collider.position[1];
        motion.camera_position = motion.collider.position;
        for actor in self.monsters.active() {
            let mut actor = actor.borrow_mut();
            let monster = &mut actor.health;
            monster.state = 0;
        }
        zones.respawn.fill(0);
        if motion.document_layout == crate::kx::Layout::Beta2004 {
            switches[2..].fill(0);
            switches[1] = 0;
        } else {
            switches[16..].fill(0);
        }
        Ok(world.zones.clone())
    }

    pub fn register_checkpoint(
        &mut self,
        index: usize,
        position: [f32; 3],
        direction: f32,
    ) -> Result<(), &'static str> {
        let checkpoint = self
            .checkpoints
            .get_mut(index)
            .ok_or("checkpoint index out of range")?;
        checkpoint.valid = true;
        checkpoint.position = [position[0], position[1], position[2], 1.];
        checkpoint.direction = direction * std::f32::consts::TAU;
        Ok(())
    }
    pub fn respawn_at(
        &self,
        index: usize,
        motion: &mut Motion,
        world: &World,
    ) -> Result<Option<Sound>, &'static str> {
        let checkpoint = self
            .checkpoints
            .get(index)
            .ok_or("checkpoint index out of range")?;
        if checkpoint.valid || index == 1 {
            motion.place(world, checkpoint.position, checkpoint.direction, 0.)?;
            Ok(Some(Sound {
                slot: 11,
                volume: 1.,
                near_distance: 0.2,
                far_distance: 1.,
            }))
        } else {
            Ok(None)
        }
    }
    pub fn continue_game(
        &mut self,
        motion: &mut Motion,
        world: &World,
        switches: &[u8; 256],
    ) -> Result<(), &'static str> {
        motion.place(world, self.parameters.start_position, 0., 0.)?;
        for (i, checkpoint) in self.checkpoints.iter().enumerate() {
            if switches[32 + i] != 0 && checkpoint.valid {
                motion.place(world, checkpoint.position, checkpoint.direction, 0.)?;
            }
        }
        for actor in self.monsters.active() {
            let mut actor = actor.borrow_mut();
            let monster = &mut actor.health;
            if monster.life > 0 {
                monster.state = 0;
            }
        }
        self.inventory.amounts[0] = 100;
        if self.inventory.amounts[4] < 200 {
            self.inventory.amounts[4] = 200;
        }
        Ok(())
    }
}
