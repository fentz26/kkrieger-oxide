//! Material11Insert's destination-alpha FXQuad operations.
use crate::{
    gpu_mesh::{EffectJob, Material},
    material::State,
};
use miniquad::RenderingBackend;

/// Original GenOverlayManager material recipes for CLRDESTALPHA/ADDDESTALPHA.
pub fn alpha_state(add: bool) -> State {
    let mut state = State::default();
    state.words[0] = 0x30002 | if add { 0x8000 } else { 0x3000 };
    state.words[28] = 0x00ffffff;
    state.words[9] = 0x10;
    state.words[23] = 8;
    state
}
pub struct Inserts {
    quad: EffectJob,
    clear: (Material, State),
    add: (Material, State),
}
impl Inserts {
    pub fn new(
        ctx: &mut dyn RenderingBackend,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let clear = alpha_state(false);
        let add = alpha_state(true);
        let clear_gpu = Material::compile_effect(ctx, &clear, depth_bits)?;
        let add_gpu = match Material::compile_effect(ctx, &add, depth_bits) {
            Ok(material) => material,
            Err(error) => {
                clear_gpu.delete(ctx);
                return Err(error);
            }
        };
        let mut quad = EffectJob::new(ctx, 4, 6)?;
        // FXQuad's Gray-code vertex order and the original sQuad indices.
        let vertices = [
            [-1., 1., 1., 0., 0., 0., 0., 0.],
            [1., 1., 1., 0., 0., 0., 0., 0.],
            [1., -1., 1., 0., 0., 0., 0., 0.],
            [-1., -1., 1., 0., 0., 0., 0., 0.],
        ];
        quad.update(ctx, &vertices, &[0, 1, 2, 0, 2, 3])?;
        Ok(Self {
            quad,
            clear: (clear_gpu, clear),
            add: (add_gpu, add),
        })
    }
    /// Draw into the currently selected viewport/target. The render loop
    /// restores its scene view-projection after this operation, as in source.
    pub fn destination_alpha(
        &self,
        ctx: &mut dyn RenderingBackend,
        add: bool,
        dimensions: [i32; 2],
    ) -> Result<(), &'static str> {
        let camera = crate::scene::camera::Environment {
            orthogonal: 1,
            ..Default::default()
        };
        let environment = camera.material_environment(dimensions)?;
        let (material, state) = if add { &self.add } else { &self.clear };
        material.draw_effect(ctx, &self.quad, &[], state, &environment)
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        self.quad.delete(ctx);
        self.clear.0.delete(ctx);
        self.add.0.delete(ctx);
    }
}
