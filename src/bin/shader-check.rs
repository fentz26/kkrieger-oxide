//! Offline GPU compilation check; not part of the game executable.
use kkrieger_oxide::{
    kx::{Document, Layout},
    material,
};
use miniquad::*;
fn main() {
    let beta = std::env::args().any(|v| v == "--beta");
    let data = if beta {
        std::fs::read(".reference/verified-beta.kx").unwrap()
    } else {
        include_bytes!("../../assets/kkrieger3383.kx").to_vec()
    };
    let doc = Document::parse_layout(
        &data,
        if beta {
            Layout::Beta2004
        } else {
            Layout::Legacy3383
        },
    )
    .unwrap();
    let roots: Vec<_> = doc
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(doc.classes[op.class].id, 0xd0 | 0xd1))
        .map(|(i, _)| i)
        .collect();
    let materials = material::generate_many(&doc, &roots).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut states = Vec::new();
    for material in materials {
        for pass in &material.passes {
            if seen.insert(std::rc::Rc::as_ptr(&pass.shader)) {
                states.push((pass.shader.operator, pass.shader.state.borrow().clone()));
            }
        }
    }
    miniquad::start(
        conf::Conf {
            window_title: "kkrieger-oxide shader check".into(),
            window_width: 64,
            window_height: 64,
            ..Default::default()
        },
        move || {
            let mut ctx = window::new_rendering_backend();
            verify_uploads(&mut *ctx);
            verify_draw(&mut *ctx);
            let mut count = 0;
            for (id, state) in &states {
                let source = state
                    .shader_sources()
                    .unwrap_or_else(|e| panic!("material {id}: {e}"));
                let meta = ShaderMeta {
                    images: source.images.iter().map(|(name, _)| name.clone()).collect(),
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("vc", UniformType::Float4).array(24),
                            UniformDesc::new("pc", UniformType::Float4).array(8),
                            UniformDesc::new("fog_color", UniformType::Float4),
                        ],
                    },
                };
                match ctx.new_shader(
                    ShaderSource::Glsl {
                        vertex: &source.vertex,
                        fragment: &source.fragment,
                    },
                    meta,
                ) {
                    Ok(shader) => {
                        ctx.delete_shader(shader);
                        count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "material {id}: {e:?}\n{}\n{}",
                            source.vertex, source.fragment
                        );
                        std::process::exit(1);
                    }
                }
            }
            println!(
                "{count} original material shader programs compiled on {:?}",
                ctx.info().backend
            );
            window::quit();
            Box::new(Check)
        },
    );
}
struct Check;
impl EventHandler for Check {
    fn update(&mut self) {}
    fn draw(&mut self) {}
}

#[cfg(target_os = "macos")]
fn verify_uploads(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        bitmap::Bitmap,
        gpu_texture,
        texture_upload::{Encoding, Pyramid},
    };
    use miniquad::native::gl::*;
    #[link(name = "OpenGL", kind = "framework")]
    unsafe extern "C" {
        fn glGetTexImage(
            target: u32,
            level: i32,
            format: u32,
            kind: u32,
            pixels: *mut std::ffi::c_void,
        );
    }
    for format in [1, 5] {
        for threshold in [0, 16] {
            for flags in 0..8 {
                let mut bitmap = Bitmap::flat(3, 2, 0, 0);
                bitmap.format = format;
                bitmap.mip_threshold = threshold;
                for (i, p) in bitmap.pixels.iter_mut().enumerate() {
                    *p = std::array::from_fn(|c| ((i * 38473 + c * 18763) & 65535) as u16);
                }
                let pyramid = Pyramid::from_bitmap(&bitmap).unwrap();
                let texture = gpu_texture::upload(ctx, &pyramid, flags).unwrap();
                unsafe {
                    let RawId::OpenGl(raw) = ctx.texture_raw_id(texture) else {
                        unreachable!()
                    };
                    let mut previous = 0;
                    glGetIntegerv(0x8069, &mut previous);
                    glBindTexture(GL_TEXTURE_2D, raw);
                    for (i, level) in pyramid.levels.iter().enumerate() {
                        let mut bytes = vec![0u8; level.pixels.len()];
                        glGetTexImage(
                            GL_TEXTURE_2D,
                            i as i32,
                            GL_RGBA,
                            if pyramid.encoding == Encoding::Rgba8 {
                                GL_UNSIGNED_BYTE
                            } else {
                                GL_BYTE
                            },
                            bytes.as_mut_ptr().cast(),
                        );
                        for (&actual, &expected) in bytes.iter().zip(&level.pixels) {
                            if pyramid.encoding == Encoding::Rgba8 {
                                assert_eq!(actual, expected);
                            } else {
                                assert_eq!((actual as i8).max(-127), (expected as i8).max(-127));
                            }
                        }
                    }
                    glBindTexture(GL_TEXTURE_2D, previous as u32);
                    assert_eq!(glGetError(), GL_NO_ERROR);
                }
                ctx.delete_texture(texture);
            }
        }
    }
    let (cube, attenuation) = gpu_texture::lighting_lookups(ctx);
    ctx.delete_texture(cube);
    ctx.delete_texture(attenuation);
    unsafe {
        assert_eq!(glGetError(), GL_NO_ERROR);
    }
    println!("32 unsigned/signed texture uploads and all written mip levels read back correctly");
}
#[cfg(not(target_os = "macos"))]
fn verify_uploads(_: &mut dyn RenderingBackend) {}

fn verify_draw(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        gpu_mesh,
        material::{Program, State, uniforms::Environment},
        mesh::{Mesh, identity},
        prepared_mesh::PreparedMesh,
    };
    let mut mesh = Mesh::cube([1, 1, 1], 0, [0.5, 0.5, 0.5, 0., 0., 0., 0., 0., 0.75]);
    for v in &mut mesh.vertices {
        v.attributes[3] = [64. / 255., 32. / 255., 16. / 255., 128. / 255.];
    }
    let prepared = PreparedMesh::from_mesh(&mut mesh, &[vec![], vec![Program::Static]]).unwrap();
    assert_eq!(
        std::mem::size_of::<kkrieger_oxide::prepared_mesh::Vertex>(),
        48
    );
    let job = gpu_mesh::StaticJob::upload(ctx, &prepared, 0).unwrap();
    let mut state = State::default();
    state.words[19] = 0x10;
    let material = gpu_mesh::Material::compile(ctx, &state, 16).unwrap();
    let color = ctx.new_render_texture(TextureParams {
        width: 64,
        height: 64,
        format: TextureFormat::RGBA8,
        ..Default::default()
    });
    let depth = ctx.new_render_texture(TextureParams {
        width: 64,
        height: 64,
        format: TextureFormat::Depth,
        ..Default::default()
    });
    let pass = ctx.new_render_pass(color, Some(depth));
    let environment = Environment {
        model: identity(),
        view_projection: identity(),
        world_to_camera: identity(),
        camera: identity(),
        light_position: [0., 0., 0., 1.],
        light_color: [1.; 4],
        light_amplify: 1.,
        light_range: 0.,
        fog_start: 0.,
        fog_end: 1.,
        fog_color: 0,
    };
    ctx.begin_pass(Some(pass), PassAction::default());
    material.draw(ctx, &job, &[], &state, &environment).unwrap();
    ctx.end_render_pass();
    let mut pixels = vec![0; 64 * 64 * 4];
    ctx.texture_read_pixels(color, &mut pixels);
    let packed = prepared.vertices[0].color;
    assert_eq!(
        &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        &[(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255]
    );
    material.delete(ctx);
    job.delete(ctx);
    ctx.delete_render_pass(pass);
    println!("Original static mesh draw and packed vertex colors read back correctly");
}
