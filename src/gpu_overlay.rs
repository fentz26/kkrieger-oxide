//! Original IPP copy, blur, color, merge and mask materials on double-UV GPU quads.
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
pub fn material_state(material: usize, color: u32) -> Result<State, &'static str> {
    if matches!(material, 25 | 26) {
        let mut state = State::default();
        state.words[0] = 0x20002 | if material == 26 { 0x2000 } else { 0 };
        state.words[4] = 0x101;
        state.words[9] = 0x10;
        state.words[11] = 0xb0;
        state.words[28] = 0x00408040;
        state.textures[0] = Some(0);
        if material == 25 {
            state.words[5] = 0x111;
            state.words[12] = 0x40;
            state.textures[1] = Some(1);
        } else {
            state.words[6] = 0x111;
            state.words[15] = 0xa0;
            state.words[17] = 0x40;
            state.words[30] = 0x00ffffff;
            state.textures[2] = Some(2);
        }
        Ok(state)
    } else if (8..=24).contains(&material) {
        let mut state = State::default();
        let operation = if material < 16 {
            material - 8
        } else {
            if material == 24 { 0 } else { material - 16 }
        };
        let combiner = [0x20, 0x80, 0xa0, 0x30, 0x40, 0x50, 0x98, 0xb0][operation];
        state.words[0] = 0x20002;
        state.words[4] = 0x101;
        state.words[28] = color;
        state.words[15] = 0x60;
        state.textures[0] = Some(0);
        if material < 16 {
            state.words[9] = 0x10;
            state.words[11] = combiner;
        } else {
            state.words[11] = 0x10;
            state.words[12] = if material == 24 { 0x195 } else { combiner };
            state.words[5] = 0x111;
            state.textures[1] = Some(1);
        }
        Ok(state)
    } else if matches!(material, 4 | 5) {
        let mut state = State::default();
        state.words[0] = 0x20002 | if material == 5 { 0x2000 } else { 0 };
        state.words[4] = 0x101;
        state.words[5] = 0x111;
        state.words[11] = 0x18;
        state.words[12] = 0x28;
        state.words[23] = 0xe0;
        state.words[28] = color;
        state.textures = [Some(0), Some(1), None, None];
        Ok(state)
    } else {
        let mode = if material == 6 {
            0
        } else {
            material
                .checked_sub(27)
                .ok_or("overlay material has not been ported")?
        };
        copy_state(mode, color)
    }
}
pub struct Overlays {
    depth_bits: u32,
    quad: EffectJob,
    materials: BTreeMap<usize, Material>,
}
impl Overlays {
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
        self.draw_sources(ctx, draw, &[source])
    }
    pub fn draw_sources(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        draw: &Draw,
        sources: &[TextureId],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = material_state(draw.material, draw.color)?;
        if let Some(amplify) = draw.color2 {
            state.words[30] = amplify;
        }
        let supplied = [Some(draw.texture), draw.texture1, draw.texture2, None];
        if sources.len() != state.textures.iter().flatten().count()
            || supplied
                .iter()
                .zip(&state.textures)
                .any(|(supplied, required)| supplied.is_some() != required.is_some())
        {
            return Err("overlay source slots do not match original material".into());
        }
        let mode = if draw.material == 6 {
            27
        } else {
            draw.material
        };
        if let std::collections::btree_map::Entry::Vacant(entry) = self.materials.entry(mode) {
            entry.insert(Material::compile_double(ctx, &state, self.depth_bits)?);
        }
        let mut vertices = draw.vertices;
        for vertex in &mut vertices {
            vertex[5] = 1. - vertex[5];
            vertex[7] = 1. - vertex[7];
        }
        self.quad.update(ctx, &vertices, &[0, 1, 2, 0, 2, 3])?;
        for (&source, (stage, _)) in sources.iter().zip(
            state
                .textures
                .iter()
                .enumerate()
                .filter(|(_, texture)| texture.is_some()),
        ) {
            ctx.texture_set_filter(source, FilterMode::Linear, MipmapFilterMode::None);
            let wrap = if state.words[4 + stage] & 0x100 != 0 {
                TextureWrap::Clamp
            } else {
                TextureWrap::Repeat
            };
            ctx.texture_set_wrap(source, wrap, wrap);
        }
        let [x0, y0, x1, y1] = draw.viewport.window;
        let environment = draw.environment.material_environment([x1 - x0, y1 - y0])?;
        self.materials[&mode].draw_double(ctx, &self.quad, sources, &state, &environment)?;
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        self.quad.delete(ctx);
        for (_, material) in self.materials {
            material.delete(ctx);
        }
    }
}
