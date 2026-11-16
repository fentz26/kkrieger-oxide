//! Original sMaterialEnv values; projection/viewport consumption is separate.
use crate::mesh::{Matrix, identity};
#[derive(Clone, Debug)]
pub struct Environment {
    pub light_position: [f32; 4],
    pub light_color: [f32; 4],
    pub light_range: f32,
    pub light_amplify: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub fog_color: u32,
    pub model: Matrix,
    pub space: Matrix,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub center_x: f32,
    pub center_y: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub orthogonal: i32,
}
impl Default for Environment {
    fn default() -> Self {
        Self {
            light_position: [0.; 4],
            light_color: [0.; 4],
            light_range: 0.,
            light_amplify: 0.,
            fog_start: 0.,
            fog_end: 4096.,
            fog_color: 0xff808080,
            model: identity(),
            space: identity(),
            zoom_x: 1.,
            zoom_y: 1.,
            center_x: 0.,
            center_y: 0.,
            near_clip: 0.125,
            far_clip: 4096.,
            orthogonal: 0,
        }
    }
}

/// Original Exec_Scene_Rotate input transform.
pub fn rotate(parent: Matrix, camera: Matrix, direction: [f32; 3], axis: i32) -> Matrix {
    let unit = |mut v: [f32; 4], safe: bool| {
        let length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        if safe && (length.is_nan() || length <= 1e-20) {
            v[..3].copy_from_slice(&[1., 0., 0.]);
        } else {
            let scale = (1. / f64::from(length).sqrt()) as f32;
            for n in &mut v[..3] {
                *n *= scale;
            }
        }
        v
    };
    let cross = |a: [f32; 4], b: [f32; 4]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
            0.,
        ]
    };
    let v = unit([direction[0], direction[1], direction[2], 0.], true);
    let mut matrix = camera;
    match axis {
        0 => {
            matrix[0] = v;
            matrix[2] = unit(cross(matrix[0], parent[1]), false);
            matrix[1] = unit(cross(matrix[2], matrix[0]), false);
        }
        1 => {
            matrix[1] = v;
            matrix[0] = unit(cross(matrix[1], parent[2]), false);
            matrix[2] = unit(cross(matrix[0], matrix[1]), false);
        }
        2 => {
            matrix[2] = v;
            matrix[1] = unit(cross(matrix[2], parent[0]), false);
            matrix[0] = unit(cross(matrix[1], matrix[2]), false);
        }
        _ => {}
    }
    matrix[3] = parent[3];

    matrix
}

impl Environment {
    /// Original intro-build projection. The renderer applies API clip-space
    /// conversion separately; these values also drive engine portal culling.
    pub fn projection(&self, viewport: [i32; 2]) -> Result<Matrix, &'static str> {
        let shift_x = 1. / viewport[0] as f32;
        let shift_y = 1. / viewport[1] as f32;
        match self.orthogonal {
            0 => Ok([
                [self.zoom_x, 0., 0., 0.],
                [0., self.zoom_y, 0., 0.],
                [self.center_x, self.center_y, 1., 1.],
                [-shift_x, shift_y, -self.near_clip, 0.],
            ]),
            1 => Ok([
                [self.zoom_x, 0., 0., 0.],
                [0., self.zoom_y, 0., 0.],
                [0., 0., 1. / self.far_clip, 0.],
                [-shift_x, shift_y, 0., 1.],
            ]),
            _ => Err("projection mode not supported by the original intro build"),
        }
    }
}
