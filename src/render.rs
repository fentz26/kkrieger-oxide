use crate::{
    math,
    procedural::{Room, TILE, TILES, texture_atlas},
    synth::Audio,
};
use miniquad::*;
use std::time::Instant;

#[repr(C)]
struct Uniforms {
    mvp: math::Mat4,
    eye: [f32; 3],
    time: f32,
    flash: f32,
}

pub struct App {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    room: Room,
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    keys: [bool; 8],
    grabbed: bool,
    last: Instant,
    start: Instant,
    time: f32,
    flash: f32,
    fps: f32,
    audio: Audio,
    hud: crate::hud::Hud,
}
impl App {
    pub fn new() -> Self {
        let start = Instant::now();
        let room = Room::generate();
        let atlas = texture_atlas();
        let mut ctx = window::new_rendering_backend();
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&room.vertices),
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&room.indices),
        );
        let texture = ctx.new_texture_from_rgba8((TILE * TILES) as u16, TILE as u16, &atlas);
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                ShaderMeta {
                    images: vec!["atlas".into()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("eye", UniformType::Float3),
                            UniformDesc::new("time", UniformType::Float1),
                            UniformDesc::new("flash", UniformType::Float1),
                        ],
                    },
                },
            )
            .expect("world shader compilation failed");
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("normal", VertexFormat::Float3),
                VertexAttribute::new("uv", VertexFormat::Float2),
                VertexAttribute::new("material", VertexFormat::Float1),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
        );
        let hud = crate::hud::Hud::new(&mut *ctx);
        let audio = Audio::new();
        eprintln!(
            "Generated {} triangles, {} KiB texture atlas in {:.2} ms. Audio: {}.",
            room.indices.len() / 3,
            atlas.len() / 1024,
            start.elapsed().as_secs_f64() * 1000.,
            audio.available()
        );
        eprintln!(
            "WASD move | arrows look | click capture/fire | shift sprint | M mute | R reset | Esc release | Q quit"
        );
        Self {
            ctx,
            pipeline,
            bindings: Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: vec![texture],
            },
            room,
            eye: [0., 1.65, 3.5],
            yaw: 0.,
            pitch: 0.,
            keys: [false; 8],
            grabbed: false,
            last: Instant::now(),
            start,
            time: 0.,
            flash: 0.,
            fps: 60.,
            audio,
            hud,
        }
    }
    fn capture(&mut self, grab: bool) {
        self.grabbed = grab;
        window::set_cursor_grab(grab);
        window::show_mouse(!grab);
        if !grab {
            self.keys.fill(false);
        }
    }
    fn key(key: KeyCode) -> Option<usize> {
        match key {
            KeyCode::W => Some(0),
            KeyCode::S => Some(1),
            KeyCode::A => Some(2),
            KeyCode::D => Some(3),
            KeyCode::Left => Some(4),
            KeyCode::Right => Some(5),
            KeyCode::LeftShift => Some(6),
            KeyCode::RightShift => Some(7),
            _ => None,
        }
    }
}
impl EventHandler for App {
    fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f32();
        let dt = elapsed.min(0.05);
        self.last = now;
        self.time = self.start.elapsed().as_secs_f32();
        self.fps = self.fps * 0.95 + 0.05 / elapsed.max(0.0001);
        self.yaw += (self.keys[5] as i32 - self.keys[4] as i32) as f32 * dt * 1.6;
        let forward = (self.keys[0] as i32 - self.keys[1] as i32) as f32;
        let strafe = (self.keys[3] as i32 - self.keys[2] as i32) as f32;
        let norm = (forward * forward + strafe * strafe).sqrt().max(1.);
        let speed = if self.keys[6] || self.keys[7] {
            5.
        } else {
            2.7
        };
        let dx = (self.yaw.sin() * forward + self.yaw.cos() * strafe) * dt * speed / norm;
        let dz = (-self.yaw.cos() * forward + self.yaw.sin() * strafe) * dt * speed / norm;
        // Bounded substeps prevent sprint movement from tunneling through narrow pillars.
        let steps = ((dx.abs().max(dz.abs()) / 0.08).ceil() as usize).max(1);
        for _ in 0..steps {
            let x = self.eye[0] + dx / steps as f32;
            if self.room.can_stand(x, self.eye[2]) {
                self.eye[0] = x;
            }
            let z = self.eye[2] + dz / steps as f32;
            if self.room.can_stand(self.eye[0], z) {
                self.eye[2] = z;
            }
        }
        self.flash = (self.flash - dt * 6.).max(0.);
    }
    fn draw(&mut self) {
        let (w, h) = window::screen_size();
        if w < 1. || h < 1. {
            return;
        }
        let mvp = math::mul(
            math::perspective(w / h),
            math::view(self.eye, self.yaw, self.pitch),
        );
        self.ctx
            .begin_default_pass(PassAction::clear_color(0.006, 0.012, 0.022, 1.));
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.apply_uniforms(UniformsSource::table(&Uniforms {
            mvp,
            eye: self.eye,
            time: self.time,
            flash: self.flash,
        }));
        self.ctx.draw(0, self.room.indices.len() as i32, 1);
        self.hud.draw(
            &mut *self.ctx,
            w,
            h,
            self.fps,
            self.grabbed,
            self.audio.available(),
            self.flash,
        );
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }
    fn key_down_event(&mut self, key: KeyCode, _mods: KeyMods, repeat: bool) {
        if let Some(i) = Self::key(key) {
            self.keys[i] = true;
        }
        if !repeat {
            match key {
                KeyCode::Escape => self.capture(false),
                KeyCode::Q => window::order_quit(),
                KeyCode::M => self.audio.toggle_mute(),
                KeyCode::R => {
                    self.eye = [0., 1.65, 3.5];
                    self.yaw = 0.;
                    self.pitch = 0.;
                }
                _ => {}
            }
        }
    }
    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        if let Some(i) = Self::key(key) {
            self.keys[i] = false;
        }
    }
    fn raw_mouse_motion(&mut self, dx: f32, dy: f32) {
        if self.grabbed {
            self.yaw += dx * 0.002;
            self.pitch = (self.pitch - dy * 0.002).clamp(-1.4, 1.4);
        }
    }
    fn mouse_button_down_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Left {
            if !self.grabbed {
                self.capture(true);
            } else {
                self.flash = 1.;
                self.audio.shoot();
            }
        }
    }
    fn window_minimized_event(&mut self) {
        self.capture(false);
    }
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec3 normal;
attribute vec2 uv;
attribute float material;
uniform mat4 mvp;
varying vec3 world;
varying vec3 n;
varying vec2 texcoord;
varying float mat;
void main() { world=position; n=normal; texcoord=uv; mat=material; gl_Position=mvp*vec4(position,1.0); }
"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
uniform sampler2D atlas;
uniform vec3 eye;
uniform float time;
uniform float flash;
varying vec3 world;
varying vec3 n;
varying vec2 texcoord;
varying float mat;
vec3 light(vec3 p,vec3 color,float power) {
    vec3 d=p-world;
    return color*(0.10+max(dot(normalize(d),n),0.0))*power/(1.0+dot(d,d)*0.20);
}
void main() {
    vec2 uv=vec2((floor(mat+0.5)+(fract(texcoord.x)*127.0+0.5)/128.0)/4.0,
                 (fract(texcoord.y)*127.0+0.5)/128.0);
    vec3 tex=texture2D(atlas,uv).rgb;
    vec3 illum=vec3(0.13,0.17,0.21);
    illum+=light(vec3(0.0,3.4,-12.0),vec3(0.12,0.75,1.0),5.0+sin(time*1.7)*0.3);
    illum+=light(vec3(0.0,4.0,0.0),vec3(0.45,0.68,0.85),3.5);
    illum+=light(vec3(0.0,4.0,-6.0),vec3(0.8,0.5,0.22),3.0);
    illum+=light(vec3(0.0,3.8,-18.0),vec3(0.18,0.55,0.9),3.5);
    illum+=light(eye,vec3(1.0,0.55,0.18),flash*7.0);
    vec3 c=tex*illum;
    if(mat>2.5) c=tex*(1.15+0.18*sin(time*2.0+world.y));
    float dist=length(eye-world);
    float fog=1.0-exp(-dist*0.027);
    c=mix(c,vec3(0.012,0.026,0.043),fog);
    gl_FragColor=vec4(pow(c,vec3(0.82)),1.0);
}
"#;
