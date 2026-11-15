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
