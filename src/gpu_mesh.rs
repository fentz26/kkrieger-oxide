//! Original static and shadow jobs on the GPU. Sprite and thick-line finalizers
//! require their own expansion handlers; they are not approximated here.
use crate::{
    material::{self, Program, State, uniforms::Environment},
    prepared_mesh::PreparedMesh,
};
use miniquad::*;

pub struct StaticJob {
    pub vertex: BufferId,
    pub index: BufferId,
    pub index_count: i32,
}
impl StaticJob {
    pub fn upload(
        ctx: &mut dyn RenderingBackend,
        mesh: &PreparedMesh,
        job: usize,
    ) -> Result<Self, &'static str> {
        let job = mesh.jobs.get(job).ok_or("mesh job index out of range")?;
        if job.program != Program::Static {
            return Err("requested mesh job requires a different original finalizer");
        }
        let vertices: Vec<_> = job
            .vertices
            .iter()
            .map(|&i| mesh.vertices[i].clone())
            .collect();
        if vertices.is_empty() || job.indices.is_empty() {
            return Err("static job has no triangles");
        }
        let vertex = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let index = if vertices.len() <= 65536 {
            let indices: Vec<_> = job.indices.iter().map(|&i| i as u16).collect();
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&indices),
            )
        } else {
            let indices: Vec<_> = job.indices.iter().map(|&i| i as u32).collect();
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&indices),
            )
        };
        Ok(Self {
            vertex,
            index,
            index_count: job.indices.len() as i32,
        })
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vertex);
        ctx.delete_buffer(self.index);
    }
}

pub struct Material {
    pub shader: ShaderId,
    pub pipeline: Pipeline,
    pub images: Vec<(String, material::shader::Image)>,
    shadow_mode: Option<bool>,
    depth_bits: u32,
}
impl Material {
    pub fn compile(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::compile_kind(ctx, state, depth_bits, None)
    }
    pub fn compile_shadow(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
        zfail: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if state.words[0] & 0x40000 == 0 {
            return Err("shadow material is missing source extrusion flag".into());
        }
        Self::compile_kind(ctx, state, depth_bits, Some(zfail))
    }
    fn compile_kind(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
        shadow: Option<bool>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if ctx.info().backend != Backend::OpenGl {
            return Err("original material rendering requires OpenGL".into());
        }
        let source = state.shader_sources()?;
        let shader = ctx.new_shader(
            ShaderSource::Glsl {
                vertex: &source.vertex,
                fragment: &source.fragment,
            },
            ShaderMeta {
                images: source.images.iter().map(|(name, _)| name.clone()).collect(),
                uniforms: UniformBlockLayout {
                    uniforms: vec![
                        UniformDesc::new("vc", UniformType::Float4).array(24),
                        UniformDesc::new("pc", UniformType::Float4).array(8),
                        UniformDesc::new("fog_color", UniformType::Float4),
                    ],
                },
            },
        )?;
        let attributes = if shadow.is_some() {
            vec![VertexAttribute::new("position", VertexFormat::Float4)]
        } else {
            vec![
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("normal", VertexFormat::Float3),
                VertexAttribute::new("tangent", VertexFormat::Float3),
                VertexAttribute::new("color", VertexFormat::Byte4),
                VertexAttribute::new("uv0", VertexFormat::Float2),
            ]
        };
        let mut pipeline_state = state.clone();
        if let Some(zfail) = shadow {
            pipeline_state.words[0] =
                (pipeline_state.words[0] & !0xf00006) | 0x500002 | if zfail { 0x800000 } else { 0 };
        }
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &attributes,
            shader,
            pipeline_state.pipeline_params(depth_bits),
        );
        Ok(Self {
            shader,
            pipeline,
            images: source.images,
            shadow_mode: shadow,
            depth_bits,
        })
    }
    pub fn draw(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &StaticJob,
        images: &[TextureId],
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if self.shadow_mode.is_some() {
            return Err("shadow material requires shadow vertex layout");
        }
        if images.len() != self.images.len() {
            return Err("material texture binding count mismatch");
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, images);
        let constants = state.constants(environment);
        ctx.apply_uniforms(UniformsSource::table(&constants.uniforms));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn draw_shadow(
        &self,
        ctx: &mut dyn RenderingBackend,
        job: &ShadowJob,
        state: &State,
        environment: &Environment,
    ) -> Result<(), &'static str> {
        if self.shadow_mode.is_none() {
            return Err("shadow draw requires an original shadow material");
        }
        if job.index_count == 0 {
            return Ok(());
        }
        if self.shadow_mode != Some(job.zfail) {
            return Err("shadow cap mode does not match stencil pipeline");
        }
        if !self.images.is_empty() {
            return Err("original shadow shader must not sample textures");
        }
        ctx.apply_pipeline(&self.pipeline);
        apply_depth(state, self.depth_bits);
        ctx.apply_bindings_from_slice(&[job.vertex], job.index, &[]);
        ctx.apply_uniforms(UniformsSource::table(
            &state.constants(environment).uniforms,
        ));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_pipeline(self.pipeline);
        ctx.delete_shader(self.shader);
    }
}

pub struct ShadowJob {
    vertex: BufferId,
    index: BufferId,
    index_count: i32,
    vertex_count: usize,
    index_capacity: usize,
    big_indices: bool,
    zfail: bool,
}
impl ShadowJob {
    pub fn new(
        ctx: &mut dyn RenderingBackend,
        volume: &crate::shadow::Volume,
        index_capacity: usize,
    ) -> Result<Self, &'static str> {
        if volume.vertices.is_empty() {
            return Err("shadow job has no vertices");
        }
        let vertex = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&volume.vertices),
        );
        let big_indices = volume.vertices.len() >= 65536;
        let index_capacity = index_capacity.max(3);
        let index = if big_indices {
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u32>(index_capacity),
            )
        } else {
            ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u16>(index_capacity),
            )
        };
        Ok(Self {
            vertex,
            index,
            index_count: 0,
            vertex_count: volume.vertices.len(),
            index_capacity,
            big_indices,
            zfail: false,
        })
    }
    pub fn update(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        volume: &crate::shadow::Volume,
        zfail: bool,
    ) -> Result<(), &'static str> {
        if volume.vertices.len() != self.vertex_count {
            return Err("shadow vertex count changed");
        }
        let indices = volume.draw_indices(zfail);
        if indices.len() > self.index_capacity || indices.iter().any(|&i| i >= self.vertex_count) {
            return Err("shadow index storage exceeded");
        }
        if !indices.is_empty() {
            if self.big_indices {
                let data: Vec<_> = indices.iter().map(|&i| i as u32).collect();
                ctx.buffer_update(self.index, BufferSource::slice(&data));
            } else {
                let data: Vec<_> = indices.iter().map(|&i| i as u16).collect();
                ctx.buffer_update(self.index, BufferSource::slice(&data));
            }
        }
        self.index_count = indices.len() as i32;
        self.zfail = zfail;
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vertex);
        ctx.delete_buffer(self.index);
    }
}

// Miniquad 0.4.11 uses depth_write to enable GL_DEPTH_TEST and never applies
// glDepthMask or polygon bias. Original light/shadow passes need read-only depth.
fn apply_depth(state: &State, depth_bits: u32) {
    use miniquad::native::gl::*;
    let params = state.pipeline_params(depth_bits);
    unsafe {
        glEnable(GL_DEPTH_TEST);
        glDepthFunc(params.depth_test.into());
        glDepthMask(u8::from(params.depth_write));
        if let Some((factor, units)) = params.depth_write_offset {
            glEnable(GL_POLYGON_OFFSET_FILL);
            glPolygonOffset(factor, units);
        } else {
            glDisable(GL_POLYGON_OFFSET_FILL);
        }
    }
}
