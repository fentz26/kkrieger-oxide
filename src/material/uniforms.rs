//! Register constants from the original sMaterial11::Set implementation.
use super::State;
use crate::mesh::{Matrix, srt_matrix};
use crate::scene::exec::multiply;

pub struct Environment {
    pub model: Matrix,
    pub view_projection: Matrix,
    pub world_to_camera: Matrix,
    pub camera: Matrix,
    pub light_position: [f32; 4],
    pub light_color: [f32; 4],
    pub light_amplify: f32,
    pub light_range: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub fog_color: u32,
}
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Uniforms {
    pub vc: [[f32; 4]; 24],
    pub pc: [[f32; 4]; 8],
    pub fog_color: [f32; 4],
}
#[derive(Clone, Debug)]
pub struct Constants {
    pub uniforms: Uniforms,
    pub vertex_count: usize,
    pub pixel_count: usize,
}
fn transpose(a: Matrix) -> Matrix {
    std::array::from_fn(|r| std::array::from_fn(|c| a[c][r]))
}
pub(crate) fn mul4(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            b[0][c] * a[r][0] + b[1][c] * a[r][1] + b[2][c] * a[r][2] + b[3][c] * a[r][3]
        })
    })
}
fn rotate34(m: Matrix, v: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|c| m[0][c] * v[0] + m[1][c] * v[1] + m[2][c] * v[2] + m[3][c])
}
pub(crate) fn trans_r(m: Matrix) -> Matrix {
    let mut result = m;
    for (r, row) in result.iter_mut().enumerate().take(3) {
        for (c, value) in row.iter_mut().enumerate().take(3) {
            *value = m[c][r];
        }
    }
    let v = m[3].map(|v| -v);
    let translation: [f32; 3] =
        std::array::from_fn(|c| result[0][c] * v[0] + result[1][c] * v[1] + result[2][c] * v[2]);
    result[3][..3].copy_from_slice(&translation);
    result
}
fn color(value: u32) -> [f32; 4] {
    [
        (value >> 16) & 255,
        (value >> 8) & 255,
        value & 255,
        value >> 24,
    ]
    .map(|v| v as f32 / 255.)
}
impl State {
    pub fn has_texture_srt(&self) -> bool {
        const STAGES: [u32; 4] = [3, 4, 9, 10];
        const ALPHA: [u32; 16] = [
            0,
            1,
            1,
            1,
            1 << 3,
            1 << 4,
            1 << 9,
            1 << 10,
            1 << 1,
            1 << 2,
            1 << 7,
            1 << 8,
            0,
            0,
            0,
            0,
        ];
        let mut mask = 0;
        for i in 0..13 {
            let flags = self.words[8 + i];
            mask |= ALPHA[(flags & 15) as usize];
            if flags & 0xff0 != 0 {
                mask |= 1 << i;
            }
        }
        let alpha = self.words[23];
        mask |= ALPHA[(alpha & 15) as usize] | ALPHA[((alpha >> 4) & 15) as usize];
        if self.words[2] & 7 == 5 && self.textures[1].is_some() {
            mask |= 1 << 4;
        }
        STAGES
            .iter()
            .enumerate()
            .any(|(i, &stage)| mask & (1 << stage) != 0 && self.words[4 + i] & 0x3000 >= 0x2000)
    }
    /// Unwritten source registers are zero-filled. Only registers consumed by
    /// the compiled program are meaningful; pc[1].y is unused by the source.
    pub fn constants(&self, env: &Environment) -> Constants {
        let w = &self.words;
        let f = |i| f32::from_bits(w[i]);
        let mut result = Constants {
            uniforms: Uniforms {
                vc: [[0.; 4]; 24],
                pc: [[0.; 4]; 8],
                fog_color: color(env.fog_color),
            },
            vertex_count: 0,
            pixel_count: 0,
        };
        let vc = &mut result.uniforms.vc;
        let pc = &mut result.uniforms.pc;
        vc[..4].copy_from_slice(&transpose(mul4(env.model, env.view_projection)));
        let inverse = trans_r(env.model);
        vc[4] = rotate34(inverse, env.light_position);
        if w[0] & 0x40000 != 0 {
            vc[4][3] = 0.;
            result.vertex_count = 5;
            return result;
        }
        vc[5] = if env.light_range != 0. {
            let mut v = vc[4].map(|v| v * (-0.5 / env.light_range));
            for a in &mut v[..3] {
                *a += 0.5;
            }
            v[3] = 0.5 / env.light_range;
            v
        } else {
            [0.5, 0.5, 0.5, 0.]
        };
        vc[6] = rotate34(inverse, env.camera[3]);
        vc[7] = std::array::from_fn(|i| f(24 + i));
        result.vertex_count = 8;
        if self.has_texture_srt() {
            let srt = std::array::from_fn(|i| f(34 + i));
            let m = transpose(srt_matrix(&srt));
            vc[8] = m[0];
            vc[9] = m[1];
            let (sin, cos) = f(45).sin_cos();
            vc[10] = [f(43) * cos, f(44) * sin, 0., f(46)];
            vc[11] = [-f(43) * sin, f(44) * cos, 0., f(47)];
            result.vertex_count = 12;
        }
        if w[1] & 0x30 != 0 {
            vc[12..16].copy_from_slice(&transpose(multiply(env.model, env.world_to_camera)));
            result.vertex_count = 16;
        }
        if w[1] & 0x700 != 0 {
            let m = match w[1] & 0x700 {
                0x200 => Some(env.model),
                0x300 => Some(multiply(env.model, env.world_to_camera)),
                _ => None,
            };
            if let Some(m) = m {
                vc[16..20].copy_from_slice(&transpose(m));
            }
            result.vertex_count = 20;
        }
        if w[0] & 0x40 != 0 {
            let m = transpose(multiply(env.model, env.world_to_camera));
            let mut fog = m[2];
            fog[3] -= env.fog_end;
            vc[20] = fog.map(|v| v * (1. / (env.fog_start - env.fog_end)));
            result.vertex_count = 21;
        }
        if w[2] & 7 == 0 {
            for i in 0..4 {
                pc[i] = color(w[28 + i]);
            }
            result.pixel_count = 4;
        } else {
            pc[0] = color(w[28]);
            for (v, &light) in pc[0].iter_mut().zip(&env.light_color) {
                *v *= light;
                *v *= env.light_amplify;
            }
            let power = f(32);
            let use_power = power.clamp(8., 60.);
            let inv_scale = 1. / 7.336032;
            pc[1][0] = power;
            pc[1][2] = inv_scale + (use_power - 8.) * 0.01565302;
            pc[1][3] = inv_scale - pc[1][2];
            result.pixel_count = 2;
        }
        result
    }
}
