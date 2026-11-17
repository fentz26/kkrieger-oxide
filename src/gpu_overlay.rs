//! Original GENOVER_TEX1 and six IPP Copy materials on the double-UV quad path.
use crate::{
    gpu_mesh::{EffectJob, Material},
    material::State,
    scene::overlay::Draw,
};
use miniquad::{FilterMode, MipmapFilterMode, RenderingBackend, TextureId, TextureWrap};
use std::collections::BTreeMap;
pub fn copy_state(mode: usize, color: u32) -> Result<State, &'static str> {
    let blend = *[0, 0x2000, 0x3000, 0x4000, 0x5000, 0x1000]
        .get(mode)
        .ok_or("original copy mode out of range")?;
    let mut state = State::default();
    state.words[0] = 0x20002 | blend;
    state.words[28] = color;
    state.words[9] = 0x10;
    state.words[11] = 0x40;
    state.words[23] = 0x84;
    state.textures[0] = Some(0);
    Ok(state)
}
pub struct Copies {
    depth_bits: u32,
    quad: EffectJob,
    materials: BTreeMap<usize, Material>,
}
impl Copies {
    pub fn new(ctx: &mut dyn RenderingBackend, depth_bits: u32) -> Result<Self, &'static str> {
        Ok(Self {
            depth_bits,
            quad: EffectJob::new(ctx, 4, 6)?,
            materials: Default::default(),
        })
    }
    /// The caller selects the original destination viewport first. The source
    /// is a GPU render target, whose vertical coordinates differ from bitmaps.
    pub fn draw(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        draw: &Draw,
        source: TextureId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if draw.texture1.is_some() || draw.texture2.is_some() {
            return Err("multi-texture overlay requires its original material".into());
        }
        let mode = if draw.material == 6 {
            0
        } else {
            draw.material
                .checked_sub(27)
                .ok_or("overlay is not an original copy material")?
        };
        let state = copy_state(mode, draw.color)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = self.materials.entry(mode) {
            entry.insert(Material::compile_double(ctx, &state, self.depth_bits)?);
        }
        let mut vertices = draw.vertices;
        for vertex in &mut vertices {
            vertex[5] = 1. - vertex[5];
            vertex[7] = 1. - vertex[7];
        }
        self.quad.update(ctx, &vertices, &[0, 1, 2, 0, 2, 3])?;
        ctx.texture_set_filter(source, FilterMode::Linear, MipmapFilterMode::None);
        ctx.texture_set_wrap(source, TextureWrap::Repeat, TextureWrap::Repeat);
        let [x0, y0, x1, y1] = draw.viewport.window;
        let environment = draw.environment.material_environment([x1 - x0, y1 - y0])?;
        self.materials[&mode].draw_double(ctx, &self.quad, &[source], &state, &environment)?;
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        self.quad.delete(ctx);
        for (_, material) in self.materials {
            material.delete(ctx);
        }
    }
}
