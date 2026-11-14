//! Vector rectangles and a built-in 5x7 font keep the HUD free of font/image assets.
use miniquad::*;
const CAPACITY: usize = 32000;
#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}
pub struct Hud {
    pipeline: Pipeline,
    bindings: Bindings,
    vertices: Vec<Vertex>,
}
impl Hud {
    pub fn new(ctx: &mut dyn RenderingBackend) -> Self {
        let shader=ctx.new_shader(ShaderSource::Glsl {
            vertex:"#version 100\nattribute vec2 pos; attribute vec4 color; varying lowp vec4 tint; void main(){gl_Position=vec4(pos,0.,1.);tint=color;}",
            fragment:"#version 100\nprecision mediump float; varying lowp vec4 tint; void main(){gl_FragColor=tint;}",
        },ShaderMeta { images:vec![],uniforms:UniformBlockLayout {uniforms:vec![]} }).expect("HUD shader compilation failed");
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("color", VertexFormat::Float4),
            ],
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );
        let vb = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<Vertex>(CAPACITY),
        );
        let indices: Vec<u16> = (0..CAPACITY as u16).collect();
        let ib = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );
        Self {
            pipeline,
            bindings: Bindings {
                vertex_buffers: vec![vb],
                index_buffer: ib,
                images: vec![],
            },
            vertices: Vec::with_capacity(CAPACITY),
        }
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4], screen: [f32; 2]) {
        for [px, py] in [
            [x, y],
            [x + w, y],
            [x + w, y + h],
            [x, y],
            [x + w, y + h],
            [x, y + h],
        ] {
            self.vertices.push(Vertex {
                pos: [px / screen[0] * 2. - 1., 1. - py / screen[1] * 2.],
                color: c,
            });
        }
    }
    fn text(&mut self, text: &str, x: f32, y: f32, scale: f32, c: [f32; 4], screen: [f32; 2]) {
        for (i, ch) in text.chars().enumerate() {
            for (row, bits) in glyph(ch).into_iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.rect(
                            x + (i * 6 + col) as f32 * scale,
                            y + row as f32 * scale,
                            scale,
                            scale,
                            c,
                            screen,
                        );
                    }
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        w: f32,
        h: f32,
        fps: f32,
        grabbed: bool,
        audio: bool,
        flash: f32,
    ) {
        self.vertices.clear();
        // Logical HUD coordinates keep typography stable on high-DPI displays.
        let screen = [960., h / w * 960.];
        let sh = screen[1];
        let cyan = [0.32, 0.83, 0.92, 1.];
        let dim = [0.47, 0.63, 0.68, 1.];
        let white = [0.85, 0.92, 0.92, 1.];
        self.rect(24., 24., 3., 38., cyan, screen);
        self.text("KKRIEGER / OXIDE", 38., 25., 2.3, white, screen);
        self.text("01   PROCEDURAL SYSTEMS LAB", 39., 49., 1., cyan, screen);
        self.text(
            &format!("{:03.0} FPS", fps.min(999.)),
            840.,
            27.,
            1.4,
            cyan,
            screen,
        );
        self.text("RUST RUNTIME", 840., 45., 1., dim, screen);
        self.rect(24., sh - 75., 912., 1., [0.24, 0.55, 0.61, 0.5], screen);
        self.text("SECTOR 01", 25., sh - 60., 1.2, dim, screen);
        self.text("REACTOR CHAMBER", 25., sh - 42., 1.8, white, screen);
        self.text(
            "WASD MOVE  /  ARROWS LOOK  /  SHIFT SPRINT",
            350.,
            sh - 57.,
            1.05,
            dim,
            screen,
        );
        self.text(
            "CLICK CAPTURE + FIRE  /  ESC RELEASE  /  Q QUIT",
            350.,
            sh - 39.,
            1.05,
            dim,
            screen,
        );
        self.text(
            if audio { "M AUDIO" } else { "NO AUDIO" },
            840.,
            sh - 57.,
            1.1,
            cyan,
            screen,
        );
        self.text("R RESET", 840., sh - 39., 1.1, dim, screen);
        let (cx, cy) = (480., sh * 0.5);
        let c = if flash > 0. {
            [1., 0.7, 0.3, 1.]
        } else {
            [0.55, 0.85, 0.88, 0.7]
        };
        for (x, y, ww, hh) in [
            (cx - 10., cy, 6., 1.),
            (cx + 5., cy, 6., 1.),
            (cx, cy - 10., 1., 6.),
            (cx, cy + 5., 1., 6.),
        ] {
            self.rect(x, y, ww, hh, c, screen);
        }
        if !grabbed {
            self.rect(
                325.,
                sh - 118.,
                310.,
                26.,
                [0.01, 0.035, 0.05, 0.85],
                screen,
            );
            self.text(
                "CLICK TO ENTER THE CHAMBER",
                345.,
                sh - 110.,
                1.6,
                cyan,
                screen,
            );
        }
        // Minimal first-person emitter silhouette, kept in screen space for this milestone.
        self.rect(625., sh - 135., 76., 59., [0.035, 0.05, 0.06, 1.], screen);
        self.rect(640., sh - 174., 40., 79., [0.12, 0.16, 0.18, 1.], screen);
        self.rect(648., sh - 191., 24., 25., [0.04, 0.07, 0.085, 1.], screen);
        self.rect(653., sh - 183., 14., 5., cyan, screen);
        if flash > 0. {
            self.rect(646., sh - 207., 28., 16., [1., 0.65, 0.2, flash], screen);
        }
        assert!(self.vertices.len() <= CAPACITY);
        ctx.buffer_update(
            self.bindings.vertex_buffers[0],
            BufferSource::slice(&self.vertices),
        );
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.draw(0, self.vertices.len() as i32, 1);
    }
}

fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '/' => [1, 1, 2, 4, 8, 16, 16],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        _ => [0; 7],
    }
}
