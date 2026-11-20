//! OpenGL upload of original texture bytes and mip levels. Miniquad's generic
//! formats do not include signed RGBA8, so that storage is specified explicitly.
use crate::texture_upload::{Encoding, Pyramid};
use miniquad::{native::gl::*, *};

pub fn upload(
    ctx: &mut dyn RenderingBackend,
    pyramid: &Pyramid,
    flags: u32,
) -> Result<TextureId, &'static str> {
    if ctx.info().backend != Backend::OpenGl {
        return Err("original texture upload requires OpenGL");
    }
    let base = pyramid.levels.first().ok_or("empty texture pyramid")?;
    let (filter, mipmap_filter) = match flags & 7 {
        1 => (FilterMode::Linear, MipmapFilterMode::None),
        2 | 3 => (FilterMode::Linear, MipmapFilterMode::Linear),
        4 => (FilterMode::Nearest, MipmapFilterMode::Nearest),
        5 => (FilterMode::Linear, MipmapFilterMode::Nearest),
        _ => (FilterMode::Nearest, MipmapFilterMode::None),
    };
    let texture = ctx.new_texture(
        TextureAccess::Static,
        TextureSource::Bytes(&base.pixels),
        TextureParams {
            width: base.width as u32,
            height: base.height as u32,
            format: TextureFormat::RGBA8,
            wrap: if flags & 0x100 != 0 {
                TextureWrap::Clamp
            } else {
                TextureWrap::Repeat
            },
            min_filter: filter,
            mag_filter: filter,
            mipmap_filter,
            ..Default::default()
        },
    );
    // The context owns this texture. Bind and restore the current unit without
    // changing Miniquad's cached binding or active texture-unit bookkeeping.
    let error = unsafe {
        #[cfg(target_vendor = "apple")]
        let RawId::OpenGl(raw) = ctx.texture_raw_id(texture) else {
            unreachable!()
        };
        #[cfg(not(target_vendor = "apple"))]
        let RawId::OpenGl(raw) = ctx.texture_raw_id(texture);
        let mut previous = 0;
        glGetIntegerv(0x8069, &mut previous);
        glBindTexture(GL_TEXTURE_2D, raw);
        for level in 0..pyramid.allocated_levels {
            let data = pyramid.levels.get(level);
            let width = (base.width >> level).max(1);
            let height = (base.height >> level).max(1);
            let (internal, kind) = match pyramid.encoding {
                Encoding::Rgba8 => (GL_RGBA8, GL_UNSIGNED_BYTE),
                Encoding::Rgba8Snorm => (GL_RGBA8_SNORM, GL_BYTE),
            };
            glTexImage2D(
                GL_TEXTURE_2D,
                level as i32,
                internal as i32,
                width as i32,
                height as i32,
                0,
                GL_RGBA,
                kind,
                data.map_or(std::ptr::null(), |l| l.pixels.as_ptr().cast()),
            );
        }
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_BASE_LEVEL, 0);
        glTexParameteri(
            GL_TEXTURE_2D,
            GL_TEXTURE_MAX_LEVEL,
            (pyramid.allocated_levels - 1) as i32,
        );
        if flags & 7 == 3 {
            glTexParameterf(GL_TEXTURE_2D, 0x84fe, 4.0);
        }
        glBindTexture(GL_TEXTURE_2D, previous as u32);
        glGetError()
    };
    if error != GL_NO_ERROR {
        ctx.delete_texture(texture);
        return Err("OpenGL rejected original texture storage or sampling state");
    }
    Ok(texture)
}

pub fn lighting_lookups(ctx: &mut dyn RenderingBackend) -> (TextureId, TextureId) {
    let faces = crate::material::lookup::normalizer();
    let levels: Vec<_> = faces.iter().map(|face| vec![face.as_slice()]).collect();
    let slices: Vec<_> = levels.iter().map(Vec::as_slice).collect();
    let normalizer = ctx.new_texture(
        TextureAccess::Static,
        TextureSource::Array(&slices),
        TextureParams {
            kind: TextureKind::CubeMap,
            width: 64,
            height: 64,
            format: TextureFormat::RGBA8,
            wrap: TextureWrap::Clamp,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        },
    );
    let attenuation = ctx.new_texture(
        TextureAccess::Static,
        TextureSource::Bytes(&crate::material::lookup::attenuation()),
        TextureParams {
            width: 32,
            height: 1024,
            format: TextureFormat::RGBA8,
            wrap: TextureWrap::Clamp,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        },
    );
    (normalizer, attenuation)
}

/// Offscreen RGBA8 + D24S8 target for the original stencil lighting passes.
/// The returned pass owns both attachments; delete it with delete_render_pass.
pub fn depth_stencil_pass(
    ctx: &mut dyn RenderingBackend,
    width: u32,
    height: u32,
) -> Result<(RenderPass, TextureId), &'static str> {
    if ctx.info().backend != Backend::OpenGl {
        return Err("original stencil target requires OpenGL");
    }
    let color = ctx.new_render_texture(TextureParams {
        width,
        height,
        format: TextureFormat::RGBA8,
        ..Default::default()
    });
    let depth = ctx.new_render_texture(TextureParams {
        width,
        height,
        format: TextureFormat::Depth,
        ..Default::default()
    });
    unsafe {
        #[cfg(target_vendor = "apple")]
        let RawId::OpenGl(raw) = ctx.texture_raw_id(depth) else {
            unreachable!()
        };
        #[cfg(not(target_vendor = "apple"))]
        let RawId::OpenGl(raw) = ctx.texture_raw_id(depth);
        let mut previous = 0;
        glGetIntegerv(0x8069, &mut previous);
        glBindTexture(GL_TEXTURE_2D, raw);
        glTexImage2D(
            GL_TEXTURE_2D,
            0,
            0x88f0,
            width as i32,
            height as i32,
            0,
            0x84f9,
            0x84fa,
            std::ptr::null(),
        );
        glBindTexture(GL_TEXTURE_2D, previous as u32);
    }
    let pass = ctx.new_render_pass(color, Some(depth));
    ctx.begin_pass(Some(pass), PassAction::Nothing);
    let valid = unsafe {
        #[cfg(target_vendor = "apple")]
        let RawId::OpenGl(raw) = ctx.texture_raw_id(depth) else {
            unreachable!()
        };
        #[cfg(not(target_vendor = "apple"))]
        let RawId::OpenGl(raw) = ctx.texture_raw_id(depth);
        glFramebufferTexture2D(GL_FRAMEBUFFER, 0x8d20, GL_TEXTURE_2D, raw, 0);
        glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE
            && glGetError() == GL_NO_ERROR
    };
    ctx.end_render_pass();
    if !valid {
        ctx.delete_render_pass(pass);
        return Err("OpenGL rejected the original stencil target");
    }
    Ok((pass, color))
}

/// D3D Clear ignores draw write masks. Preserve that behavior while
/// restoring GL draw state so Miniquad's state cache remains consistent.
/// Scissor still clips Clear, as in Direct3D 9:
/// https://learn.microsoft.com/en-us/windows/win32/direct3d9/scissor-test
fn with_clear_state(
    ctx: &mut dyn RenderingBackend,
    operation: impl FnOnce(&mut dyn RenderingBackend),
) {
    if ctx.info().backend != Backend::OpenGl {
        operation(ctx);
        return;
    }
    unsafe {
        let mut depth = 0;
        let mut color = [0; 4];
        let mut front = 0;
        let mut back = 0;
        glGetIntegerv(0x0b72, &mut depth);
        glGetIntegerv(0x0c23, color.as_mut_ptr());
        glGetIntegerv(0x0b98, &mut front);
        glGetIntegerv(0x8ca5, &mut back);
        glDepthMask(1);
        glColorMask(1, 1, 1, 1);
        glStencilMaskSeparate(GL_FRONT, u32::MAX);
        glStencilMaskSeparate(GL_BACK, u32::MAX);
        operation(ctx);
        glDepthMask(depth as u8);
        glColorMask(
            color[0] as u8,
            color[1] as u8,
            color[2] as u8,
            color[3] as u8,
        );
        glStencilMaskSeparate(GL_FRONT, front as u32);
        glStencilMaskSeparate(GL_BACK, back as u32);
    }
}
pub fn begin_pass(ctx: &mut dyn RenderingBackend, pass: Option<RenderPass>, action: PassAction) {
    if matches!(action, PassAction::Nothing) {
        ctx.begin_pass(pass, action);
    } else {
        with_clear_state(ctx, |ctx| ctx.begin_pass(pass, action));
    }
}
pub fn clear(
    ctx: &mut dyn RenderingBackend,
    color: Option<(f32, f32, f32, f32)>,
    depth: Option<f32>,
    stencil: Option<i32>,
) {
    with_clear_state(ctx, |ctx| ctx.clear(color, depth, stencil));
}

/// Static authored bitmaps may be bound at multiple material stages with
/// different sampling flags. OpenGL texture sampling belongs to the object,
/// so those bindings need distinct GPU objects while equal bindings share one.
#[derive(Default)]
pub struct Cache {
    images:
        std::collections::BTreeMap<(usize, u32), (std::rc::Rc<crate::bitmap::Bitmap>, TextureId)>,
    lookups: Option<(TextureId, TextureId)>,
}
impl Cache {
    pub fn resource_counts(&self) -> (usize, usize) {
        (
            self.images.len(),
            if self.lookups.is_some() { 2 } else { 0 },
        )
    }
    pub fn resolve(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        state: &crate::material::State,
        image: crate::material::shader::Image,
        bitmaps: &std::collections::BTreeMap<usize, std::rc::Rc<crate::bitmap::Bitmap>>,
    ) -> Result<TextureId, &'static str> {
        use crate::material::shader::Image;
        match image {
            Image::Texture { operator, stage } => {
                if state.textures.get(stage).copied().flatten() != Some(operator) {
                    return Err("material texture stage does not match shader binding");
                }
                let bitmap = bitmaps
                    .get(&operator)
                    .ok_or("material bitmap has not been prepared")?;
                let flags = state.words[4 + stage] & 0x107;
                let key = (std::rc::Rc::as_ptr(bitmap) as usize, flags);
                if let Some((_, texture)) = self.images.get(&key) {
                    return Ok(*texture);
                }
                let pyramid = crate::texture_upload::Pyramid::from_bitmap(bitmap)?;
                let texture = upload(ctx, &pyramid, flags)?;
                self.images
                    .insert(key, (std::rc::Rc::clone(bitmap), texture));
                Ok(texture)
            }
            Image::Normalizer | Image::Attenuation => {
                let (normalizer, attenuation) =
                    *self.lookups.get_or_insert_with(|| lighting_lookups(ctx));
                Ok(if image == Image::Normalizer {
                    normalizer
                } else {
                    attenuation
                })
            }
        }
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        for (_, (_, texture)) in self.images {
            ctx.delete_texture(texture);
        }
        if let Some((normalizer, attenuation)) = self.lookups {
            ctx.delete_texture(normalizer);
            ctx.delete_texture(attenuation);
        }
    }
}
