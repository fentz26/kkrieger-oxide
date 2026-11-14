//! Original static mesh jobs on the GPU. Other finalizers require their original
//! expansion/visibility handlers and are rejected here rather than approximated.
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
}
impl Material {
    pub fn compile(
        ctx: &mut dyn RenderingBackend,
        state: &State,
        depth_bits: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("normal", VertexFormat::Float3),
                VertexAttribute::new("tangent", VertexFormat::Float3),
                // Miniquad passes integer attributes without normalization. The GLSL
                // decoder performs D3DCOLOR's BGRA swizzle and exact /255 conversion.
                VertexAttribute::new("color", VertexFormat::Byte4),
                VertexAttribute::new("uv0", VertexFormat::Float2),
            ],
            shader,
            state.pipeline_params(depth_bits),
        );
        Ok(Self {
            shader,
            pipeline,
            images: source.images,
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
        if images.len() != self.images.len() {
            return Err("material texture binding count mismatch");
        }
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&Bindings {
            vertex_buffers: vec![job.vertex],
            index_buffer: job.index,
            images: images.to_vec(),
        });
        let constants = state.constants(environment);
        ctx.apply_uniforms(UniformsSource::table(&constants.uniforms));
        ctx.draw(0, job.index_count, 1);
        Ok(())
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_pipeline(self.pipeline);
        ctx.delete_shader(self.shader);
    }
}
