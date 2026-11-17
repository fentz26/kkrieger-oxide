//! Execute the original scene paint plan into an already selected GPU viewport.
use crate::{
    bitmap::{Bitmap, FontPage},
    gpu_insert, gpu_mesh, gpu_texture,
    kx::Document,
    material::{self, Program},
    scene::{
        culling::Frustum,
        effects,
        exec::{Execution, Job},
        light, paint,
        viewport::RenderPass,
    },
};
use miniquad::RenderingBackend;
use std::{collections::BTreeMap, rc::Rc};

pub struct Resources {
    pub paint: paint::Assets,
    pub bitmaps: BTreeMap<usize, Rc<Bitmap>>,
    pub pages: BTreeMap<u8, FontPage>,
}
pub struct Options {
    pub usage_mask: u32,
    pub light_capacity: usize,
    pub current_render: [bool; 256],
    pub fps: f32,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            usage_mask: u32::MAX,
            light_capacity: 4,
            current_render: [false; 256],
            fps: 0.,
        }
    }
}
#[derive(Default, Debug)]
pub struct Drawn {
    pub meshes: usize,
    pub shadows: usize,
    pub standard_batches: usize,
    pub effects: usize,
}
pub struct Scene {
    pub resources: Resources,
    meshes: gpu_mesh::PaintCache,
    standard: gpu_mesh::StandardCache,
    effects: gpu_mesh::EffectCache,
    textures: gpu_texture::Cache,
    inserts: gpu_insert::Inserts,
}
impl Scene {
    pub fn new(
        ctx: &mut dyn RenderingBackend,
        resources: Resources,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            resources,
            meshes: gpu_mesh::PaintCache::new(depth_bits),
            standard: gpu_mesh::StandardCache::new(depth_bits),
            effects: gpu_mesh::EffectCache::new(depth_bits),
            textures: Default::default(),
            inserts: gpu_insert::Inserts::new(ctx, depth_bits)?,
        })
    }
    /// The caller selects/begins the original render target and viewport first.
    /// Lights are supplied after the chosen source/beta selection profile.
    pub fn draw(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        execution: &mut Execution,
        doc: &Document<'_>,
        pass: &RenderPass,
        lights: &[light::Prepared],
        options: &Options,
    ) -> Result<Drawn, Box<dyn std::error::Error>> {
        use paint::render::Action;
        if pass.paint_mode != 0 {
            return Err("simple/debug scene paint mode has not been connected".into());
        }
        let [x0, y0, x1, y1] = pass.viewport.window;
        let dimensions = [x1 - x0, y1 - y0];
        let mut viewport = [0; 4];
        unsafe {
            miniquad::native::gl::glGetIntegerv(0x0ba2, viewport.as_mut_ptr());
        }
        if viewport[2..] != dimensions {
            return Err("scene paint viewport dimensions do not match target".into());
        }
        let mut environment = pass.camera.material_environment(dimensions)?;
        let frustum =
            Frustum::from_view_projection(environment.view_projection, [-1., -1., 1., 1.]);
        let paint_lights: Vec<_> = lights.iter().map(light::Prepared::paint_light).collect();
        let view = paint::View {
            usage_mask: options.usage_mask,
            frustum: frustum.planes[..5].try_into().unwrap(),
            lights: &paint_lights,
            light_capacity: options.light_capacity,
        };
        let frame = paint::build_static_frame(
            doc,
            &pass.jobs,
            &self.resources.paint.meshes,
            &self.resources.paint.materials,
            &view,
            |material, pass| {
                ((&material.passes[pass] as *const material::Pass as usize) >> 3) as u16
            },
        )?;
        let rendering = Rc::new(effects::Rendering {
            camera: pass.camera.clone(),
            materials: self.resources.paint.materials.clone(),
            pages: self.resources.pages.clone(),
            fps: options.fps,
        });
        let mut drawn = Drawn::default();
        for action in paint::render::actions(&frame, &options.current_render, true) {
            match action {
                Action::ClearScissor => {
                    ctx.apply_scissor_rect(viewport[0], viewport[1], viewport[2], viewport[3])
                }
                Action::ClearDestinationAlpha => {
                    self.inserts.destination_alpha(ctx, false, dimensions)?
                }
                Action::AddDestinationAlpha => {
                    self.inserts.destination_alpha(ctx, true, dimensions)?
                }
                Action::ClearStencil => gpu_texture::clear(ctx, None, None, Some(0)),
                Action::ResetShadowStates => unsafe {
                    use miniquad::native::gl::*;
                    glDisable(GL_STENCIL_TEST);
                    glEnable(GL_CULL_FACE);
                    glCullFace(GL_BACK);
                },
                // Every submission supplies its own projection constants. Thus
                // the source ApplyViewProject restoration is already explicit.
                Action::ApplyViewProjection => {}
                Action::GrabCurrentRender => {
                    return Err("current-render target capture has not been connected".into());
                }
                Action::SetLight(index) => {
                    let light = lights.get(index).ok_or("paint light index out of range")?;
                    environment.light_position =
                        [light.position[0], light.position[1], light.position[2], 1.];
                    environment.light_color = [
                        (light.color >> 16) & 255,
                        (light.color >> 8) & 255,
                        light.color & 255,
                        light.color >> 24,
                    ]
                    .map(|v| v as f32 / 255.);
                    environment.light_amplify = light.amplify;
                    environment.light_range = light.range;
                    let [l, b, r, t] = light.rect;
                    let left = (dimensions[0] as f32 * (1. + l) * 0.5) as i32;
                    let right = (dimensions[0] as f32 * (1. + r) * 0.5) as i32;
                    let top = (dimensions[1] as f32 * (1. - t) * 0.5) as i32;
                    let bottom = (dimensions[1] as f32 * (1. - b) * 0.5) as i32;
                    ctx.apply_scissor_rect(
                        viewport[0] + left,
                        viewport[1] + dimensions[1] - bottom,
                        (right - left).max(0),
                        (bottom - top).max(0),
                    );
                }
                Action::Draw(index) => {
                    let queued = &frame.jobs[index];
                    let job = &pass.jobs[queued.scene_job];
                    match &queued.kind {
                        paint::DrawKind::Effect => {
                            for leaf in
                                execution.draw_deferred_effect(doc, job, Rc::clone(&rendering))?
                            {
                                let Some(material) = leaf
                                    .material
                                    .and_then(|i| self.resources.paint.materials.get(&i))
                                else {
                                    continue;
                                };
                                drawn.effects += usize::from(self.effects.draw(
                                    ctx,
                                    &leaf.content,
                                    material,
                                    gpu_mesh::EffectView {
                                        camera: &pass.camera,
                                        dimensions,
                                    },
                                    |ctx, recipe, image| {
                                        Ok(self.textures.resolve(
                                            ctx,
                                            &recipe.state.borrow(),
                                            image,
                                            &self.resources.bitmaps,
                                        )?)
                                    },
                                )?);
                            }
                        }
                        paint::DrawKind::Mesh(paint) => {
                            let Job::Paint {
                                operator, matrix, ..
                            } = job
                            else {
                                return Err("mesh paint references a non-paint job".into());
                            };
                            let asset = self
                                .resources
                                .paint
                                .meshes
                                .get(operator)
                                .ok_or("mesh paint asset missing")?;
                            environment.model = *matrix;
                            match asset.mesh.jobs[paint.geometry_job].program {
                                Program::Static => {
                                    drawn.meshes += usize::from(self.meshes.draw_static(
                                        ctx,
                                        asset,
                                        paint,
                                        &environment,
                                        |ctx, recipe, image| {
                                            Ok(self.textures.resolve(
                                                ctx,
                                                &recipe.state.borrow(),
                                                image,
                                                &self.resources.bitmaps,
                                            )?)
                                        },
                                    )?)
                                }
                                Program::Shadow => {
                                    drawn.shadows += usize::from(self.meshes.draw_shadow(
                                        ctx,
                                        asset,
                                        paint,
                                        &environment,
                                    )?)
                                }
                                Program::Sprites | Program::ThickLines => {
                                    drawn.standard_batches += self.standard.draw(
                                        ctx,
                                        asset,
                                        paint,
                                        &environment,
                                        |ctx, recipe, image| {
                                            Ok(self.textures.resolve(
                                                ctx,
                                                &recipe.state.borrow(),
                                                image,
                                                &self.resources.bitmaps,
                                            )?)
                                        },
                                    )?
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(drawn)
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        self.meshes.delete(ctx);
        self.standard.delete(ctx);
        self.effects.delete(ctx);
        self.textures.delete(ctx);
        self.inserts.delete(ctx);
    }
}
