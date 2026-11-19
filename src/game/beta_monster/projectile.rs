//! Released scene Physic mode-0 point-projectile state.
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
