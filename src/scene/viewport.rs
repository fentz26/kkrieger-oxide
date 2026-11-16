//! Original player-build IPP viewport camera preparation, before scene execution.
use super::camera::Environment;
use crate::mesh::{Matrix, euler_matrix_fsincos, identity};
#[derive(Clone, Debug)]
pub struct Parameters {
    pub flags: u32,
    pub rotation: [f32; 3],
    pub position: [f32; 3],
    pub far_clip: f32,
    pub near_clip: f32,
    pub center: [f32; 2],
    pub zoom: [f32; 2],
    pub fog_color: u32,
    pub fog_end: f32,
    pub fog_start: f32,
}
pub struct Passes {
    pub flags: u32,
    pub cameras: Vec<Environment>,
    pub allocate_stereo_target: bool,
}
impl Parameters {
    pub fn from_words(words: &[u32]) -> Result<Self, &'static str> {
        let p = words
            .get(..18)
            .ok_or("truncated viewport camera parameters")?;
        let f = |i: usize| f32::from_bits(p[i]);
        Ok(Self {
            flags: p[1],
            rotation: [f(3), f(4), f(5)],
            position: [f(6), f(7), f(8)],
            far_clip: f(9),
            near_clip: f(10),
            center: [f(11), f(12)],
            zoom: [f(13), f(14)],
            fog_color: p[15],
            fog_end: f(16),
            fog_start: f(17),
        })
    }
    pub fn passes(
        &self,
        camera: &Environment,
        intro_stereo: bool,
        eye_distance: f32,
        focal: f32,
    ) -> Passes {
        let stereo = intro_stereo && self.flags & 0x40 != 0;
        let mut flags = self.flags;
        let mut offset = 0.;
        let mut shift = [0.; 4];
        if stereo {
            offset = 1. * eye_distance / focal;
            shift = camera.space[0];
            let [x, y, z, _] = shift;
            let length = x * x + y * y + z * z;
            if length > 1e-20 {
                let scale = (1. / f64::from(length).sqrt()) as f32;
                for n in &mut shift[..3] {
                    *n *= scale;
                }
            } else {
                shift[..3].copy_from_slice(&[1., 0., 0.]);
            }
            flags |= 3;
        }
        let cameras = (0..if stereo { 2 } else { 1 })
            .map(|pass| {
                let sign = if pass != 0 { 1. } else { -1. };
                let mut env = camera.clone();
                env.center_x = self.center[0] - sign * offset;
                let distance = sign * eye_distance * 0.5;
                for (n, shift) in env.space[3][..3].iter_mut().zip(shift) {
                    *n += shift * distance;
                }
                env
            })
            .collect();
        Passes {
            flags,
            cameras,
            allocate_stereo_target: stereo,
        }
    }
    /// Optional spline evaluation supplies the same camera and zoom values as
    /// GenSpline::Eval; it overrides the base space after projection and fog.
    pub fn camera(
        &self,
        game: &Environment,
        aspect: f32,
        spline: Option<(Matrix, f32)>,
    ) -> Environment {
        let mut env = Environment {
            far_clip: self.far_clip,
            near_clip: self.near_clip,
            center_x: self.center[0],
            center_y: self.center[1],
            zoom_x: self.zoom[0],
            zoom_y: self.zoom[1],
            ..Environment::default()
        };
        if self.flags & 4 != 0 {
            env.space = game.space;
            if self.flags & 0x300 == 0 {
                env.center_x = game.center_x;
                env.center_y = game.center_y;
                env.zoom_x = game.zoom_x;
                env.zoom_y = game.zoom_y;
                env.far_clip = game.far_clip;
                env.near_clip = game.near_clip;
            }
        } else {
            env.space = if self.rotation.iter().all(|a| a.abs() < 1e-6) {
                identity()
            } else {
                euler_matrix_fsincos(self.rotation.map(|a| a * std::f32::consts::TAU))
            };
            env.space[3][..3].copy_from_slice(&self.position);
        }
        match self.flags & 0x300 {
            0x100 => {
                env.space[1] = [0., 1., 0., 1.];
                let i = cross(env.space[1], env.space[2]);
                env.space[0][..3].copy_from_slice(&i);
                let [x, y, z, _] = env.space[0];
                let scale = (1. / f64::from(x * x + y * y + z * z).sqrt()) as f32;
                for n in &mut env.space[0][..3] {
                    *n *= scale;
                }
                let k = cross(env.space[0], env.space[1]);
                env.space[2][..3].copy_from_slice(&k);
                env.space[3] = env.space[2];
                for n in &mut env.space[3][..3] {
                    *n *= self.position[2];
                }
                env.space[3][3] = 1.;
            }
            0x200 => {
                env.space[3] = env.space[2];
                for n in &mut env.space[3][..3] {
                    *n *= self.position[2];
                }
                env.space[3][3] = 1.;
            }
            _ => {}
        }
        env.space[3][3] = 1.;
        env.fog_color = self.fog_color;
        env.fog_start = self.fog_start;
        env.fog_end = self.fog_end;
        if self.flags & 0x1000 == 0 {
            env.zoom_y *= aspect;
        }
        if let Some((space, zoom)) = spline {
            env.space = space;
            env.zoom_x *= zoom;
            env.zoom_y *= zoom;
        }
        env
    }
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// One original engine paint pass, before backend drawing and stereo blending.
#[derive(Clone, Debug)]
pub struct RenderPass {
    pub viewport: super::overlay::Viewport,
    pub camera: Environment,
    pub clear_flags: u32,
    pub clear_color: u32,
    pub paint_mode: u32,
    pub jobs: Vec<super::exec::Job>,
    pub grab_stereo_target: bool,
    pub blend_stereo_target: bool,
    pub allocate_stereo_target: bool,
}
pub fn crop(
    mut viewport: super::overlay::Viewport,
    fractions: [f32; 4],
) -> super::overlay::Viewport {
    let [x0, y0, x1, y1] = viewport.window;
    let width = x1.wrapping_sub(x0) as f32;
    let height = y1.wrapping_sub(y0) as f32;
    let window = [
        (x0 as f32 + width * fractions[0]) as i32,
        (y0 as f32 + height * fractions[1]) as i32,
        (x0 as f32 + width * fractions[2]) as i32,
        (y0 as f32 + height * fractions[3]) as i32,
    ];
    if window[0] < window[2] && window[1] < window[3] {
        viewport.window = window;
    }
    viewport
}
