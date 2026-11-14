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
        let RawId::OpenGl(raw) = ctx.texture_raw_id(texture) else {
            unreachable!()
        };
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
