//! Copy a completed GPU target to the active output viewport without changing
//! its image orientation. Scene raster conversion has already happened.
use miniquad::*;
pub struct Present {
    shader: ShaderId,
    pipeline: Pipeline,
    vertex: BufferId,
    index: BufferId,
}
impl Present {
    pub fn new(ctx: &mut dyn RenderingBackend) -> Result<Self, Box<dyn std::error::Error>> {
        if ctx.info().backend != Backend::OpenGl {
            return Err("original frame presentation requires OpenGL".into());
        }
        let shader = ctx.new_shader(ShaderSource::Glsl {
            vertex: "#version 100\nprecision highp float;attribute vec2 position;attribute vec2 uv;varying vec2 texture_uv;void main(){gl_Position=vec4(position,0.0,1.0);texture_uv=uv;}",
            fragment: "#version 100\nprecision highp float;varying vec2 texture_uv;uniform sampler2D image;void main(){gl_FragColor=texture2D(image,texture_uv);}",
        },ShaderMeta { images:vec!["image".into()], uniforms:UniformBlockLayout { uniforms:vec![] } })?;
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::Always,
                depth_write: false,
                ..Default::default()
            },
        );
        let vertices: [[f32; 4]; 4] = [
            [-1., -1., 0., 0.],
            [1., -1., 1., 0.],
            [1., 1., 1., 1.],
            [-1., 1., 0., 1.],
        ];
        let vertex = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let index = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&[0u16, 1, 2, 0, 2, 3]),
        );
        Ok(Self {
            shader,
            pipeline,
            vertex,
            index,
        })
    }
    /// Begin the destination pass before calling. The source must be a
    /// different target from the currently active color attachment.
    pub fn draw(&self, ctx: &mut dyn RenderingBackend, source: TextureId) {
        let mut viewport = [0; 4];
        unsafe {
            native::gl::glGetIntegerv(0x0ba2, viewport.as_mut_ptr());
        }
        ctx.apply_scissor_rect(viewport[0], viewport[1], viewport[2], viewport[3]);
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings_from_slice(&[self.vertex], self.index, &[source]);
        ctx.draw(0, 6, 1);
    }
    pub fn delete(self, ctx: &mut dyn RenderingBackend) {
        ctx.delete_buffer(self.vertex);
        ctx.delete_buffer(self.index);
        ctx.delete_pipeline(self.pipeline);
        ctx.delete_shader(self.shader);
    }
}
