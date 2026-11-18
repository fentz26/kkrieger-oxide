//! Released PhysicPart, with world queries separated from particle arithmetic.
use super::{
    add,
    body::{Body, Particle},
    down, mul,
};

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub gravity: f32,
    pub gravity_player: f32,
    pub damping_player_air: f32,
    pub damping_player_ground: f32,
    pub fly_mode: i32,
    pub on_ground: bool,
}
pub trait World {
    fn outside(&mut self, cell: usize, position: [f32; 4]) -> bool;
    fn find_cell(&mut self, position: [f32; 4]) -> Option<usize>;
    /// Original collision response may update particle position and history.
    fn collide(&mut self, particle: &mut Particle) -> bool;
}
impl Body {
    pub fn step_particle<W: World>(
        &mut self,
        index: usize,
        settings: Settings,
        world: &mut W,
    ) -> Result<(), &'static str> {
        let p = self
            .particles
            .get(index)
            .ok_or("particle index out of range")?;
        let kind = p.flags[3];
        let level = matches!(kind, 1 | 3);
        if level && index + 5 > self.particles.len() {
            return Err("five-particle group truncated");
        }
        if p.flags[5] != 0 {
            let old_collision = p.vectors[1];
            if p.cell.is_none_or(|cell| world.outside(cell, old_collision)) {
                let Some(cell) = world.find_cell(old_collision) else {
                    return Ok(());
                };
                self.particles[index].cell = Some(cell);
            }
            self.particles[index].flags[5] = 0;
        }
        let mut gravity = settings.gravity;
        if level {
            let mut sum = 0.;
            for p in &self.particles[index..index + 5] {
                sum = add(sum, f64::from(p.vectors[0][1]));
            }
            let mean = mul(mul(sum, f64::from(f32::from_bits(0x3e4ccccd))), 0.25);
            for p in &mut self.particles[index..index + 5] {
                p.vectors[0][1] = down(add(mul(f64::from(p.vectors[0][1]), 0.75), mean));
            }
        }
        let p = &mut self.particles[index];
        if matches!(kind, 1..=4) {
            gravity = if settings.fly_mode != 0 {
                0.
            } else {
                settings.gravity_player
            };
            p.damping = if settings.on_ground {
                settings.damping_player_ground
            } else {
                settings.damping_player_air
            };
        }
        let factor = f64::from(down(add(1., -f64::from(p.damping))));
        p.damping = 0.;
        let position = p.vectors[0];
        let history = p.vectors[2];
        let temporary: [f32; 3] = std::array::from_fn(|axis| {
            down(add(
                f64::from(position[axis]),
                -mul(factor, f64::from(history[axis])),
            ))
        });
        p.vectors[2] = position;
        for axis in 0..3 {
            let value = mul(factor, f64::from(position[axis]));
            let value = if axis == 1 {
                add(value, -f64::from(gravity))
            } else {
                value
            };
            p.vectors[0][axis] = down(add(value, f64::from(temporary[axis])));
        }
        p.vectors[0][3] = 1.;
        if world.collide(p) {
            p.damping = 0.1;
            p.vectors[2][1] = down(add(f64::from(gravity), f64::from(p.vectors[2][1])));
        }
        Ok(())
    }
}

impl Body {
    /// Final Physic phase, after particle and dynamic-cell processing.
    pub fn apply_forces(&mut self) {
        for p in &mut self.particles {
            for axis in 0..3 {
                p.vectors[0][axis] =
                    down(add(f64::from(p.vectors[0][axis]), f64::from(p.force[axis])));
            }
            p.force = [0.; 4];
        }
    }
}
