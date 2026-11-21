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
    /// Select the released beta projection for perspective and pixel-space cameras.
    pub released_projection: bool,
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
            released_projection: false,
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
    /// Default environment; beta documents use the released projection routine.
    pub fn for_layout(layout: crate::kx::Layout) -> Self {
        Self {
            released_projection: layout == crate::kx::Layout::Beta2004,
            ..Self::default()
        }
    }
    /// Feed the same camera transform into portal culling and shader constants.
    pub fn material_environment(
        &self,
        viewport: [i32; 2],
    ) -> Result<crate::material::uniforms::Environment, &'static str> {
        let world_to_camera = crate::material::uniforms::trans_r(self.space);
        let projection = self.projection(viewport)?;
        Ok(crate::material::uniforms::Environment {
            model: self.model,
            view_projection: crate::material::uniforms::mul4(world_to_camera, projection),
            world_to_camera,
            camera: self.space,
            light_position: self.light_position,
            light_color: self.light_color,
            light_amplify: self.light_amplify,
            light_range: self.light_range,
            fog_start: self.fog_start,
            fog_end: self.fog_end,
            fog_color: self.fog_color,
        })
    }
    /// Original intro-build projection. The renderer applies API clip-space
    /// conversion separately; these values also drive engine portal culling.
    pub fn projection(&self, viewport: [i32; 2]) -> Result<Matrix, &'static str> {
        if self.released_projection && matches!(self.orthogonal, 0 | 2) {
            let mut matrix = identity();
            self.released_projection_into(viewport, &mut matrix);
            return Ok(matrix);
        }
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
    /// Released MakeProjectionMatrix at `0x8022a7`. Perspective has no
    /// half-pixel offset; mode 2 maps pixels without center terms; mode 1 and
    /// unknown modes leave the destination unchanged, as the native switch does.
    pub fn released_projection_into(&self, viewport: [i32; 2], matrix: &mut Matrix) {
        match self.orthogonal {
            0 => {
                *matrix = [
                    [self.zoom_x, 0., 0., 0.],
                    [0., self.zoom_y, 0., 0.],
                    [self.center_x, self.center_y, 1., 1.],
                    [0., 0., -self.near_clip, 0.],
                ];
            }
            2 => {
                let scale = |numerator: f64, size: i32| {
                    crate::game::beta_monster::down(numerator / f64::from(size))
                };
                *matrix = [
                    [scale(2., viewport[0]), 0., 0., 0.],
                    [0., scale(-2., viewport[1]), 0., 0.],
                    [0., 0., 1., 0.],
                    [-1., 1., 0., 1.],
                ];
            }
            _ => {}
        }
    }
}
