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
            let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                verify_uploads(&mut *ctx);
                verify_draw(&mut *ctx);
                verify_shadow(&mut *ctx);
            }));
            if checked.is_err() {
                std::process::exit(1);
            }

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
    kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
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

fn verify_shadow(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        gpu_mesh, gpu_texture,
        material::{Program, State, uniforms::Environment},
        mesh::{Mesh, identity},
        prepared_mesh::PreparedMesh,
        shadow,
    };
    use miniquad::native::gl::*;
    let mut caster = Mesh::cube([1, 1, 1], 0, [1., 1., 1., 0., 0., 0., 0., 0., 2.]);
    let caster = PreparedMesh::from_mesh(
        &mut caster,
        &[vec![], vec![Program::Static, Program::Shadow]],
    )
    .unwrap();
    let mut receiver = Mesh::cube([1, 1, 1], 0, [8., 8., 0.25, 0., 0., 0., 0., 0., 5.]);
    let receiver =
        PreparedMesh::from_mesh(&mut receiver, &[vec![], vec![Program::Static]]).unwrap();
    let receiver_gpu = gpu_mesh::StaticJob::upload(ctx, &receiver, 0).unwrap();
    let caster_gpu = gpu_mesh::StaticJob::upload(ctx, &caster, 0).unwrap();
    let light = [-1.5, 1.5, 0.5];
    let mut classification = shadow::Context::default();
    let job = &caster.jobs[1];
    let volume = classification.build(&caster, job, light).unwrap();
    let mut gpu =
        gpu_mesh::ShadowJob::new(ctx, &volume, job.edges.len() * 6 + job.indices.len()).unwrap();
    let mut depth_state = State::default();
    depth_state.words[0] = 0x20322;
    depth_state.words[9] = 0x10;
    let depth_material = gpu_mesh::Material::compile(ctx, &depth_state, 24).unwrap();
    let mut shadow_state = State::default();
    shadow_state.words[0] = 0x70220;
    shadow_state.words[9] = 0x10;
    let (target, _) = gpu_texture::depth_stencil_pass(ctx, 64, 64).unwrap();
    let env = Environment {
        model: identity(),
        view_projection: [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 1.],
            [0., 0., -0.1, 0.],
        ],
        world_to_camera: identity(),
        camera: identity(),
        light_position: [light[0], light[1], light[2], 1.],
        light_color: [1.; 4],
        light_amplify: 1.,
        light_range: 16.,
        fog_start: 0.,
        fog_end: 100.,
        fog_color: 0,
    };
    let mut results = Vec::new();
    for zfail in [false, true] {
        let material = gpu_mesh::Material::compile_shadow(ctx, &shadow_state, 24, zfail).unwrap();
        gpu.update(ctx, &volume, zfail).unwrap();
        gpu_texture::begin_pass(ctx, Some(target), PassAction::default());
        // A previous stencil pass can leave a restricted write mask on clears.
        gpu_texture::clear(ctx, None, None, Some(0));
        depth_material
            .draw(ctx, &receiver_gpu, &[], &depth_state, &env)
            .unwrap();
        depth_material
            .draw(ctx, &caster_gpu, &[], &depth_state, &env)
            .unwrap();
        let mut depth_before = vec![0.0f32; 64 * 64];
        unsafe {
            glReadPixels(
                0,
                0,
                64,
                64,
                0x1902,
                GL_FLOAT,
                depth_before.as_mut_ptr().cast(),
            );
        }
        material
            .draw_shadow(ctx, &gpu, &shadow_state, &env)
            .unwrap();
        let mut depth_after = vec![0.0f32; 64 * 64];
        unsafe {
            glReadPixels(
                0,
                0,
                64,
                64,
                0x1902,
                GL_FLOAT,
                depth_after.as_mut_ptr().cast(),
            );
        }
        assert_eq!(depth_before, depth_after, "shadow pass modified depth");
        let mut stencil = vec![0u8; 64 * 64];
        unsafe {
            glReadPixels(
                0,
                0,
                64,
                64,
                0x1901,
                GL_UNSIGNED_BYTE,
                stencil.as_mut_ptr().cast(),
            );
            assert_eq!(glGetError(), GL_NO_ERROR);
        }
        assert!(
            stencil.iter().any(|&v| v != 0),
            "shadow volume did not write stencil"
        );
        results.push(stencil.iter().map(|&v| v != 0).collect::<Vec<_>>());
        ctx.end_render_pass();
        material.delete(ctx);
    }
    assert_eq!(
        results[0], results[1],
        "z-pass and z-fail disagree outside the volume"
    );
    ctx.begin_pass(Some(target), PassAction::Nothing);
    gpu_texture::clear(ctx, Some((0., 0., 0., 0.)), Some(1.), Some(0));
    ctx.apply_scissor_rect(16, 16, 16, 16);
    unsafe {
        glEnable(GL_SCISSOR_TEST);
        glDepthMask(0);
        glColorMask(0, 0, 0, 0);
        glStencilMaskSeparate(GL_FRONT, 0);
        glStencilMaskSeparate(GL_BACK, 0);
    }
    gpu_texture::clear(ctx, Some((1., 0., 0., 1.)), Some(0.25), Some(7));
    let mut colors = vec![0u8; 64 * 64 * 4];
    let mut depths = vec![0.0f32; 64 * 64];
    let mut stencils = vec![0u8; 64 * 64];
    unsafe {
        glReadPixels(
            0,
            0,
            64,
            64,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            colors.as_mut_ptr().cast(),
        );
        glReadPixels(0, 0, 64, 64, 0x1902, GL_FLOAT, depths.as_mut_ptr().cast());
        glReadPixels(
            0,
            0,
            64,
            64,
            0x1901,
            GL_UNSIGNED_BYTE,
            stencils.as_mut_ptr().cast(),
        );
        let mut depth_mask = 1;
        let mut color_mask = [1; 4];
        let mut stencil_mask = 1;
        glGetIntegerv(0x0b72, &mut depth_mask);
        glGetIntegerv(0x0c23, color_mask.as_mut_ptr());
        glGetIntegerv(0x0b98, &mut stencil_mask);
        assert_eq!(depth_mask, 0);
        assert_eq!(color_mask, [0; 4]);
        assert_eq!(stencil_mask, 0);
        glStencilMaskSeparate(GL_FRONT, u32::MAX);
        glStencilMaskSeparate(GL_BACK, u32::MAX);
        assert_eq!(glGetError(), GL_NO_ERROR);
    }
    for (x, y, inside) in [(20, 20, true), (8, 8, false)] {
        let i = y * 64 + x;
        assert_eq!(
            &colors[i * 4..i * 4 + 4],
            if inside {
                &[255, 0, 0, 255]
            } else {
                &[0, 0, 0, 0]
            }
        );
        assert_eq!(stencils[i], if inside { 7 } else { 0 });
        assert!((depths[i] - if inside { 0.25 } else { 1. }).abs() < 1e-6);
    }
    ctx.end_render_pass();
    gpu.delete(ctx);
    receiver_gpu.delete(ctx);
    caster_gpu.delete(ctx);
    depth_material.delete(ctx);
    ctx.delete_render_pass(target);
    println!("Original shadow z-pass and z-fail produce matching nonempty stencil masks");
}
