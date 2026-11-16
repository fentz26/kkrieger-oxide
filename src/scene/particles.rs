//! Original persistent particle slots and PartEmitter simulation.
use crate::{mesh::Matrix, random::Random};
/// Authored PartSystem color, size and lifetime envelopes.
pub struct Appearance {
    pub colors: [u32; 3],
    pub sizes: [f32; 3],
    pub middle: f32,
    pub fade: f32,
}
impl Appearance {
    /// Produce the original TSPACE3 quad using transposed, normalized camera
    /// axes (or the operator axes when PartSystem flag 1 is set).
    pub fn billboard(
        &self,
        particle: &Particle,
        basis: [[f32; 3]; 2],
    ) -> Result<[[f32; 8]; 4], &'static str> {
        use crate::bitmap::{color64, fade64};
        let divide = |a: i32, b: i32| {
            if b == 0 {
                return Err("particle lifetime division by zero");
            }
            i32::try_from(((a as i64) << 16) / b as i64)
                .map_err(|_| "particle lifetime division overflow")
        };
        let middle = (particle.max_time as f32 * (1022. * self.middle + 1.) / 1024.) as i32;
        let (segment, percent) = if particle.time < middle {
            (0, divide(particle.time, middle)?)
        } else {
            (
                1,
                divide(
                    particle.time.wrapping_sub(middle),
                    particle.max_time.wrapping_sub(middle),
                )?,
            )
        };
        let size = self.sizes[segment]
            + (self.sizes[segment + 1] - self.sizes[segment]) * (percent as f32 / 65536.);
        let colors = self
            .colors
            .map(|c| color64((c & 0xff00ff00) | ((c & 0xff0000) >> 16) | ((c & 0xff) << 16)));
        let color = fade64(colors[segment], colors[segment + 1], percent);
        let fadetime = (particle.max_time as f32 * (1022. * self.fade + 1.) / 1024.) as i32;
        let mut fade = 0xffff;
        if particle.time < fadetime {
            fade = divide(particle.time, fadetime)?;
        }
        if particle.time > particle.max_time.wrapping_sub(fadetime) {
            fade = divide(particle.max_time.wrapping_sub(particle.time), fadetime)?;
        }
        let color = fade64([0, 0, 0, 0xffff], color, fade);
        let packed = ((u32::from(color[3]) >> 5) & 255) << 24
            | ((u32::from(color[2]) >> 7) & 255)
            | (((u32::from(color[1]) >> 7) & 255) << 8)
            | (((u32::from(color[0]) >> 7) & 255) << 16);
        let angle = f64::from(particle.position[3]);
        let ss = angle.sin() as f32 * size;
        let sc = angle.cos() as f32 * size;
        Ok([
            (sc, ss, 0., 0.),
            (ss, -sc, 0., 1.),
            (-sc, -ss, 1., 1.),
            (-ss, sc, 1., 0.),
        ]
        .map(|(i, j, u, v)| {
            let mut vertex = [0.; 8];
            for axis in 0..3 {
                vertex[axis] = particle.position[axis] + basis[0][axis] * i;
                vertex[axis] += basis[1][axis] * j;
            }
            vertex[5] = f32::from_bits(packed);
            vertex[6] = u;
            vertex[7] = v;
            vertex
        }))
    }
}
#[derive(Clone, Debug)]
pub struct Particle {
    pub position: [f32; 4],
    pub speed: [f32; 4],
    pub time: i32,
    pub max_time: i32,
}
#[derive(Default)]
pub struct System {
    pub particles: Vec<Particle>,
    pub capacity: usize,
}
impl System {
    /// Execute the original PartSystem draw loop after its material/pass gate.
    /// The callback receives the particle before integration, including particles
    /// that expire during this draw. Invisible systems must not enter this loop.
    pub fn draw_and_advance(&mut self, slices: i32, gravity: f32, mut draw: impl FnMut(&Particle)) {
        let mut i = 0;
        while i < self.particles.len() {
            let particle = &mut self.particles[i];
            draw(particle);
            for axis in 0..4 {
                particle.position[axis] += particle.speed[axis] * slices as f32;
            }
            let triangular = slices.wrapping_mul(slices.wrapping_add(1)) / 2;
            particle.position[1] -= triangular as f32 * gravity / 31623.;
            particle.speed[1] -= slices as f32 * gravity / 31623.;
            particle.time = particle.time.wrapping_add(slices);
            if particle.time > particle.max_time {
                self.particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
}
#[derive(Default)]
pub struct Emitter {
    pub burst_count: i32,
    initialized: bool,
}
pub struct Parameters {
    pub mode: u32,
    pub burst: i32,
    pub lifetime: f32,
    pub rate: f32,
    pub speed: [f32; 4],
    pub random_position: [f32; 4],
    pub random_speed: [f32; 4],
}
impl Emitter {
    pub fn emit(
        &mut self,
        system: &mut System,
        parameters: &Parameters,
        matrix: Matrix,
        current_time: i32,
        delta: i32,
        random: &mut Random,
    ) {
        if !self.initialized {
            self.burst_count = parameters.burst;
            self.initialized = true;
        }
        let rate = parameters.rate / 1000.;
        let i = ((current_time & 0xffffff) as f32 * rate) as i32;
        let j = ((current_time.wrapping_sub(delta) & 0xffffff) as f32 * rate) as i32;
        let mut count = i
            .wrapping_sub(j)
            .min(system.capacity.saturating_sub(system.particles.len()) as i32);
        if parameters.burst != 0 {
            count = count.min(self.burst_count);
            self.burst_count = self.burst_count.wrapping_sub(count);
        }
        for _ in 0..count {
            let mut position = matrix[3];
            let mut speed = std::array::from_fn(|axis| {
                if axis == 3 {
                    parameters.speed[3]
                } else {
                    matrix[0][axis] * parameters.speed[0]
                        + matrix[1][axis] * parameters.speed[1]
                        + matrix[2][axis] * parameters.speed[2]
                }
            });
            let rv = sphere(random, parameters.mode & 1 != 0);
            if parameters.mode & 8 != 0 {
                for axis in 0..3 {
                    speed[axis] *= rv[axis];
                }
            }
            for axis in 0..3 {
                position[axis] += rv[axis] * parameters.random_position[axis];
            }
            position[3] = (random.float() * 2. - 1.) * parameters.random_position[3]
                + std::f32::consts::TAU / 8.;
            let rv = sphere(random, parameters.mode & 2 != 0);
            for axis in 0..3 {
                speed[axis] += rv[axis] * parameters.random_speed[axis];
            }
            speed[3] += (random.float() * 2. - 1.) * parameters.random_speed[3];
            for value in &mut speed {
                *value *= 0.001;
            }
            system.particles.push(Particle {
                position,
                speed,
                time: 0,
                max_time: (parameters.lifetime * 1000.) as i32,
            });
        }
    }
}
fn sphere(random: &mut Random, unit: bool) -> [f32; 3] {
    loop {
        let mut v = std::array::from_fn(|_| random.float() * 2. - 1.);
        let length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        if length <= 1. {
            if unit {
                let scale = (1. / f64::from(length).sqrt()) as f32;
                for value in &mut v {
                    *value *= scale;
                }
            }
            return v;
        }
    }
}
