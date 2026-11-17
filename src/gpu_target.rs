//! GPU attachment ownership for the original screen/render-target identities.
use crate::{gpu_texture, scene::overlay::Viewport};
use miniquad::{PassAction, RenderPass, RenderingBackend, TextureId};
use std::collections::BTreeMap;
struct Target {
    pass: RenderPass,
    color: TextureId,
    dimensions: [u32; 2],
}
#[derive(Default)]
pub struct Targets {
    targets: BTreeMap<(i32, i32), Target>,
}
impl Targets {
    pub fn len(&self) -> usize {
        self.targets.len()
    }
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
    pub fn texture(&self, screen: i32, target: i32) -> Option<TextureId> {
        self.targets
            .get(&(screen, target))
            .map(|target| target.color)
    }
    /// Attachment dimensions are the full target size. Window uses the
    /// original top-left pixel coordinates and can select a cropped viewport.
    pub fn begin(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        viewport: Viewport,
        dimensions: [u32; 2],
        clear_flags: u32,
        clear_color: u32,
    ) -> Result<TextureId, &'static str> {
        let [x0, y0, x1, y1] = viewport.window;
        if x0 < 0
            || y0 < 0
            || x1 <= x0
            || y1 <= y0
            || x1 as u32 > dimensions[0]
            || y1 as u32 > dimensions[1]
        {
            return Err("viewport window lies outside render target");
        }
        let key = (viewport.screen, viewport.render_target);
        if self
            .targets
            .get(&key)
            .is_none_or(|target| target.dimensions != dimensions)
        {
            let (pass, color) = gpu_texture::depth_stencil_pass(ctx, dimensions[0], dimensions[1])?;
            if let Some(previous) = self.targets.insert(
                key,
                Target {
                    pass,
                    color,
                    dimensions,
                },
            ) {
                // Miniquad 0.4.11 does not invalidate texture bindings on
                // deletion. Flush cached IDs while the attachments are live.
                ctx.commit_frame();
                ctx.delete_render_pass(previous.pass);
            }
        }
        let target = &self.targets[&key];
        gpu_texture::begin_pass(ctx, Some(target.pass), PassAction::Nothing);
        let y = dimensions[1] as i32 - y1;
        ctx.apply_viewport(x0, y, x1 - x0, y1 - y0);
        ctx.apply_scissor_rect(x0, y, x1 - x0, y1 - y0);
        // D3D Clear with no explicit rectangles clears the active viewport.
        // Scissor scopes OpenGL's clear to the same rectangle.
        unsafe {
            miniquad::native::gl::glEnable(miniquad::native::gl::GL_SCISSOR_TEST);
        }
        let [r, g, b, a] = [
            (clear_color >> 16) & 255,
            (clear_color >> 8) & 255,
            clear_color & 255,
            clear_color >> 24,
        ]
        .map(|v| v as f32 / 255.);
        gpu_texture::clear(
            ctx,
            (clear_flags & 1 != 0).then_some((r, g, b, a)),
            (clear_flags & 2 != 0).then_some(1.),
            (clear_flags & 6 != 0).then_some(0),
        );
        Ok(target.color)
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.commit_frame();
        for (_, target) in self.targets {
            ctx.delete_render_pass(target.pass);
        }
    }
}

/// Resolve full attachment storage, independently of a cropped draw window.
/// Screen dimensions come from the platform; IPP textures use the original pool.
pub fn attachment_dimensions(
    pool: &crate::scene::overlay::Pool,
    viewport: Viewport,
    screen: [u32; 2],
) -> Result<[u32; 2], &'static str> {
    if viewport.render_target == -1 {
        return Ok(screen);
    }
    let target = pool
        .targets
        .iter()
        .find(|target| target.texture == Some(viewport.render_target))
        .ok_or("viewport render target is not registered in IPP pool")?;
    let dimensions = crate::scene::overlay::DIMENSIONS
        .get(target.size)
        .ok_or("IPP target size out of range")?;
    Ok([dimensions[0] as u32, dimensions[1] as u32])
}
