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
                verify_texture_cache(&mut *ctx);
                verify_draw(&mut *ctx);
                verify_shadow(&mut *ctx);
                verify_inserts(&mut *ctx);
                verify_targets(&mut *ctx);
                verify_overlay_copy(&mut *ctx);
                verify_overlay_blur(&mut *ctx);
                verify_overlay_color_merge(&mut *ctx);
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

fn verify_texture_cache(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        bitmap::Bitmap,
        gpu_texture::Cache,
        material::{State, shader::Image},
    };
    use std::{collections::BTreeMap, rc::Rc};
    let bitmap = Rc::new(Bitmap {
        width: 2,
        height: 2,
        format: 1,
        mip_count: 1,
        mip_threshold: 0,
        pixels: vec![[0x0800, 0x1000, 0x2000, 0x7fff]; 4],
        font_page: None,
    });
    let bitmaps = BTreeMap::from([(42, bitmap)]);
    let mut state = State {
        textures: [Some(42), Some(42), None, None],
        ..Default::default()
    };
    state.words[4] = 0;
    state.words[5] = 0x101;
    let mut cache = Cache::default();
    let first = cache
        .resolve(
            ctx,
            &state,
            Image::Texture {
                operator: 42,
                stage: 0,
            },
            &bitmaps,
        )
        .unwrap();
    let second = cache
        .resolve(
            ctx,
            &state,
            Image::Texture {
                operator: 42,
                stage: 1,
            },
            &bitmaps,
        )
        .unwrap();
    assert_ne!(first, second);
    assert_eq!(
        cache
            .resolve(
                ctx,
                &state,
                Image::Texture {
                    operator: 42,
                    stage: 0
                },
                &bitmaps
            )
            .unwrap(),
        first
    );
    let mut pixels = [0; 16];
    for texture in [first, second] {
        ctx.texture_read_pixels(texture, &mut pixels);
        assert_eq!(pixels, [64, 32, 16, 255].repeat(4).as_slice());
    }
    assert!(
        cache
            .resolve(
                ctx,
                &state,
                Image::Texture {
                    operator: 42,
                    stage: 2
                },
                &bitmaps
            )
            .is_err()
    );
    assert_eq!(cache.resource_counts(), (2, 0));
    let normalizer = cache
        .resolve(ctx, &state, Image::Normalizer, &bitmaps)
        .unwrap();
    assert_eq!(
        cache
            .resolve(ctx, &state, Image::Normalizer, &bitmaps)
            .unwrap(),
        normalizer
    );
    assert_eq!(cache.resource_counts(), (2, 2));
    cache.delete(ctx);
    println!("Shared bitmap stages retain independent sampling and cached lighting lookups");
}

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
    // Submit the same original prepared geometry through the scene mapping.
    let cached_material = (*kkrieger_oxide::material::original_default(true)).clone();
    cached_material.passes[0]
        .shader
        .state
        .replace(state.clone());
    let recipe = std::rc::Rc::clone(&cached_material.passes[0].shader);
    let asset = kkrieger_oxide::scene::paint::Asset {
        mesh: std::rc::Rc::new(prepared),
        materials: vec![
            std::rc::Rc::new(kkrieger_oxide::material::Material::default()),
            std::rc::Rc::new(cached_material),
        ],
    };
    let mapped = kkrieger_oxide::scene::paint::MeshPaint {
        material_slot: 1,
        pass: 0,
        geometry_job: 0,
        paint: kkrieger_oxide::scene::paint::Paint {
            sort_key: 0,
            light: None,
            zfail: false,
        },
    };
    let mut cache = gpu_mesh::PaintCache::new(16);
    for _ in 0..2 {
        kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
        assert!(
            cache
                .draw_static(ctx, &asset, &mapped, &environment, |_, _, _| Err(
                    "unexpected sampled image".into()
                ))
                .unwrap()
        );
        ctx.end_render_pass();
        let mut cached_pixels = vec![0; pixels.len()];
        ctx.texture_read_pixels(color, &mut cached_pixels);
        assert_eq!(cached_pixels, pixels);
        assert_eq!(cache.resource_counts(), (1, 1));
    }
    // A mutable recipe must refresh the shader/pipeline instead of drawing stale state.
    let mut changed = State::default();
    changed.words[9] = 0x10;
    changed.words[28] = 0x0000ff00;
    recipe.state.replace(changed);
    kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
    cache
        .draw_static(ctx, &asset, &mapped, &environment, |_, _, _| {
            Err("unexpected sampled image".into())
        })
        .unwrap();
    ctx.end_render_pass();
    ctx.texture_read_pixels(color, &mut pixels);
    assert_eq!(
        &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        &[0, 255, 0, 255]
    );
    assert_eq!(cache.resource_counts(), (1, 1));
    assert_eq!(cache.compilation_count(), 2);
    recipe.state.borrow_mut().words[28] = 0x00ff0000;
    kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
    cache
        .draw_static(ctx, &asset, &mapped, &environment, |_, _, _| {
            Err("unexpected sampled image".into())
        })
        .unwrap();
    ctx.end_render_pass();
    ctx.texture_read_pixels(color, &mut pixels);
    assert_eq!(
        &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        &[255, 0, 0, 255]
    );
    assert_eq!(
        cache.compilation_count(),
        2,
        "constant animation recompiled a shader"
    );
    cache.delete(ctx);
    let mut effect = gpu_mesh::EffectJob::new(ctx, 4, 6).unwrap();
    let packed_effect = 0x80402010;
    let vertices = [
        [
            -0.25,
            0.25,
            0.75,
            0.,
            0.,
            f32::from_bits(packed_effect),
            0.,
            0.,
        ],
        [
            0.25,
            0.25,
            0.75,
            0.,
            0.,
            f32::from_bits(packed_effect),
            1.,
            0.,
        ],
        [
            -0.25,
            -0.25,
            0.75,
            0.,
            0.,
            f32::from_bits(packed_effect),
            0.,
            1.,
        ],
        [
            0.25,
            -0.25,
            0.75,
            0.,
            0.,
            f32::from_bits(packed_effect),
            1.,
            1.,
        ],
    ];
    let effect_material = gpu_mesh::Material::compile_effect(ctx, &state, 16).unwrap();
    assert!(effect.update(ctx, &vertices, &[0, 1, 4]).is_err());
    for _ in 0..2 {
        effect.update(ctx, &vertices, &[0, 1, 3, 0, 3, 2]).unwrap();
        kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
        effect_material
            .draw_effect(ctx, &effect, &[], &state, &environment)
            .unwrap();
        ctx.end_render_pass();
        ctx.texture_read_pixels(color, &mut pixels);
        assert_eq!(
            &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
            &[64, 32, 16, 255]
        );
    }
    effect.update(ctx, &[], &[]).unwrap();
    kkrieger_oxide::gpu_texture::begin_pass(
        ctx,
        Some(pass),
        PassAction::clear_color(0., 0., 0., 0.),
    );
    effect_material
        .draw_effect(ctx, &effect, &[], &state, &environment)
        .unwrap();
    ctx.end_render_pass();
    ctx.texture_read_pixels(color, &mut pixels);
    assert!(pixels.iter().all(|&byte| byte == 0));
    recipe.state.replace(state.clone());
    let mut camera = kkrieger_oxide::scene::camera::Environment {
        orthogonal: 1,
        far_clip: 1.,
        ..Default::default()
    };
    let print_environment = camera.clone();
    // Particle positions are already world-space: stale camera ModelSpace
    // must not translate them again. Print keeps its own scoped environment.
    camera.model[3][0] = 100.;
    let contents = [
        kkrieger_oxide::scene::effects::Content::Print(Box::new(
            kkrieger_oxide::scene::print::Draw {
                page: 0,
                environment: print_environment,
                geometry: kkrieger_oxide::scene::print::Geometry {
                    vertices: vertices.to_vec(),
                    indices: vec![0, 1, 3, 0, 3, 2],
                },
            },
        )),
        kkrieger_oxide::scene::effects::Content::Particles(vec![[
            vertices[0],
            vertices[1],
            vertices[3],
            vertices[2],
        ]]),
    ];
    let mut deferred = gpu_mesh::EffectCache::new(16);
    let mut print_pixels = Vec::new();
    for (index, content) in contents.iter().enumerate() {
        kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
        assert!(
            deferred
                .draw(
                    ctx,
                    content,
                    &asset.materials[1],
                    gpu_mesh::EffectView {
                        camera: &camera,
                        dimensions: [64, 64],
                    },
                    |_, _, _| Err("unexpected sampled image".into())
                )
                .unwrap()
        );
        ctx.end_render_pass();
        ctx.texture_read_pixels(color, &mut pixels);
        assert_eq!(
            &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
            &[64, 32, 16, 255]
        );
        if index == 0 {
            print_pixels = pixels.clone();
        } else {
            assert_eq!(pixels, print_pixels);
        }
        assert_eq!(deferred.resource_counts(), (1, 1));
    }
    deferred.delete(ctx);
    effect_material.delete(ctx);
    effect.delete(ctx);
    println!(
        "Original packed effect layout, streaming updates, and empty draws read back correctly"
    );

    let mut sprite_mesh = (*asset.mesh).clone();
    sprite_mesh.jobs[0].program = Program::Sprites;
    sprite_mesh.jobs[0].vertices = vec![0];
    let mut sprite_material = (*kkrieger_oxide::material::original_default(true)).clone();
    sprite_material.passes[0].program = Program::Sprites;
    sprite_material.passes[0].size = 0.5;
    let mut sprite_state = State::default();
    sprite_state.words[9] = 0x10;
    sprite_state.words[28] = 0x0000ff00;
    sprite_material.passes[0].shader.state.replace(sprite_state);
    let sprite_asset = kkrieger_oxide::scene::paint::Asset {
        mesh: std::rc::Rc::new(sprite_mesh),
        materials: vec![
            std::rc::Rc::new(kkrieger_oxide::material::Material::default()),
            std::rc::Rc::new(sprite_material),
        ],
    };
    let mut sprites = gpu_mesh::StandardCache::new(16);
    for _ in 0..2 {
        kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
        assert_eq!(
            sprites
                .draw(ctx, &sprite_asset, &mapped, &environment, |_, _, _| Err(
                    "unexpected sampled image".into()
                ))
                .unwrap(),
            1
        );
        ctx.end_render_pass();
        ctx.texture_read_pixels(color, &mut pixels);
        assert_eq!(
            &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
            &[0, 255, 0, 255]
        );
        assert_eq!(sprites.resource_counts(), (1, 1));
    }
    let mut line_mesh = (*sprite_asset.mesh).clone();
    line_mesh.jobs[0].program = Program::ThickLines;
    line_mesh.jobs[0].vertices = vec![0, 1];
    line_mesh.vertices[0].position = [-0.5, 0., 0.75];
    line_mesh.vertices[1].position = [0.5, 0., 0.75];
    let mut line_material = (*sprite_asset.materials[1]).clone();
    line_material.passes[0].program = Program::ThickLines;
    line_material.passes[0].size = 0.125;
    line_material.passes[0].aspect = 2.;
    let line_asset = kkrieger_oxide::scene::paint::Asset {
        mesh: std::rc::Rc::new(line_mesh),
        materials: vec![
            std::rc::Rc::clone(&sprite_asset.materials[0]),
            std::rc::Rc::new(line_material),
        ],
    };
    kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
    assert_eq!(
        sprites
            .draw(ctx, &line_asset, &mapped, &environment, |_, _, _| Err(
                "unexpected sampled image".into()
            ))
            .unwrap(),
        1
    );
    ctx.end_render_pass();
    ctx.texture_read_pixels(color, &mut pixels);
    assert_eq!(
        &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        &[0, 255, 0, 255]
    );
    assert_eq!(sprites.resource_counts(), (1, 1));
    sprites.delete(ctx);
    println!(
        "Original standard sprite/line layouts and authored paint submission read back correctly"
    );
    let mut doc = Document::parse(include_bytes!("../../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.operators[0].class = doc
        .classes
        .iter()
        .position(|class| class.id == 0x81)
        .unwrap();
    doc.operators[0].inputs.clear();
    doc.operators[0].links.clear();
    let mut scene_material = (*asset.materials[1]).clone();
    scene_material.passes[0].usage = kkrieger_oxide::material::Usage::Base;
    let scene_asset = kkrieger_oxide::scene::paint::Asset {
        mesh: std::rc::Rc::clone(&asset.mesh),
        materials: vec![
            std::rc::Rc::clone(&asset.materials[0]),
            std::rc::Rc::new(scene_material),
        ],
    };
    doc.operators[1].class = doc
        .classes
        .iter()
        .position(|class| class.id == 0x64)
        .unwrap();
    doc.operators[1].inputs.clear();
    doc.operators[1].links = vec![Some(2)];
    doc.operators[1].animation = &[1];
    doc.operators[1].parameters = vec![
        0,
        1,
        u32::MAX,
        u32::MAX,
        u32::MAX,
        0.25f32.to_bits(),
        0.25f32.to_bits(),
        0.25f32.to_bits(),
        0.5f32.to_bits(),
        0,
        0,
        0,
    ];
    let mut particle_material = (*kkrieger_oxide::material::original_default(true)).clone();
    particle_material.passes[0].usage = kkrieger_oxide::material::Usage::Other;
    particle_material.passes[0].shader.state.borrow_mut().words[28] = 0x0000ff00;
    let resources = kkrieger_oxide::gpu_scene::Resources {
        paint: kkrieger_oxide::scene::paint::Assets {
            meshes: std::collections::BTreeMap::from([(0, scene_asset)]),
            materials: std::collections::BTreeMap::from([(2, std::rc::Rc::new(particle_material))]),
            default_material: kkrieger_oxide::material::original_default(true),
        },
        bitmaps: Default::default(),
        pages: Default::default(),
    };
    let mut scene = kkrieger_oxide::gpu_scene::Scene::new(ctx, resources, 16).unwrap();
    let mut execution = kkrieger_oxide::scene::exec::Execution::new(&doc);
    execution.particles[0]
        .particles
        .push(kkrieger_oxide::scene::particles::Particle {
            position: [0., 0., 0.25, 0.],
            speed: [0.; 4],
            time: 50,
            max_time: 100,
        });
    let mut render_pass = kkrieger_oxide::scene::viewport::RenderPass {
        viewport: kkrieger_oxide::scene::overlay::Viewport {
            screen: 0,
            render_target: 0,
            window: [0, 0, 64, 64],
        },
        camera: kkrieger_oxide::scene::camera::Environment {
            orthogonal: 1,
            far_clip: 1.,
            ..Default::default()
        },
        clear_flags: 0,
        clear_color: 0,
        paint_mode: 0,
        jobs: vec![kkrieger_oxide::scene::exec::Job::Paint {
            operator: 0,
            matrix: identity(),
            time: 0.,
            pass_adjust: 0,
            effect_variables: None,
        }],
        grab_stereo_target: false,
        blend_stereo_target: false,
        allocate_stereo_target: false,
    };
    render_pass
        .jobs
        .push(kkrieger_oxide::scene::exec::Job::Paint {
            operator: 1,
            matrix: identity(),
            time: 0.,
            pass_adjust: 0,
            effect_variables: None,
        });
    for (z, expected) in [(0.25, [0, 255, 0, 255]), (0.75, [64, 32, 16, 255])] {
        execution.particles[0].particles[0].position[2] = z;
        kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
        let drawn = scene
            .draw(
                ctx,
                &mut execution,
                &doc,
                &render_pass,
                &[],
                &Default::default(),
            )
            .unwrap();
        assert_eq!(drawn.meshes, 1);
        assert_eq!(drawn.effects, 1);
        ctx.end_render_pass();
        ctx.texture_read_pixels(color, &mut pixels);
        assert_eq!(
            &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
            &expected
        );
    }
    render_pass.paint_mode = 1;
    kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(pass), PassAction::default());
    let drawn = scene
        .draw(
            ctx,
            &mut execution,
            &doc,
            &render_pass,
            &[],
            &Default::default(),
        )
        .unwrap();
    assert_eq!(drawn.meshes, 0);
    assert_eq!(drawn.effects, 1);
    ctx.end_render_pass();
    ctx.texture_read_pixels(color, &mut pixels);
    assert_eq!(
        &pixels[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        &[0, 255, 0, 255]
    );
    scene.delete(ctx);
    println!("Combined scene paint planner and GPU executor read back correctly");
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
    let mut mapped_material = (*kkrieger_oxide::material::original_default(true)).clone();
    let mut shadow_pass = kkrieger_oxide::material::original_default(true).passes[0].clone();
    shadow_pass.program = Program::Shadow;
    shadow_pass.usage = kkrieger_oxide::material::Usage::Shadow;
    shadow_pass.shader.state.replace(shadow_state.clone());
    mapped_material.passes.push(shadow_pass);
    let asset = kkrieger_oxide::scene::paint::Asset {
        mesh: std::rc::Rc::new(caster),
        materials: vec![
            std::rc::Rc::new(kkrieger_oxide::material::Material::default()),
            std::rc::Rc::new(mapped_material),
        ],
    };
    let mut cache = gpu_mesh::PaintCache::new(24);
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
        // The authored paint mapping must produce the same stencil as the
        // separately uploaded source volume, including both cap modes.
        gpu_texture::clear(ctx, None, None, Some(0));
        let mapped = kkrieger_oxide::scene::paint::MeshPaint {
            material_slot: 1,
            pass: 1,
            geometry_job: 1,
            paint: kkrieger_oxide::scene::paint::Paint {
                sort_key: 0x00030000,
                light: Some(0),
                zfail,
            },
        };
        assert!(cache.draw_shadow(ctx, &asset, &mapped, &env).unwrap());
        let mut cached_stencil = vec![0u8; stencil.len()];
        unsafe {
            glReadPixels(
                0,
                0,
                64,
                64,
                0x1901,
                GL_UNSIGNED_BYTE,
                cached_stencil.as_mut_ptr().cast(),
            );
            assert_eq!(glGetError(), GL_NO_ERROR);
        }
        assert_eq!(cached_stencil, stencil);
        assert_eq!(
            cache.shadow_resource_counts(),
            (1, if zfail { 2 } else { 1 })
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
    cache.delete(ctx);
    gpu.delete(ctx);
    receiver_gpu.delete(ctx);
    caster_gpu.delete(ctx);
    depth_material.delete(ctx);
    ctx.delete_render_pass(target);
    println!("Original shadow z-pass and z-fail produce matching nonempty stencil masks");
}

fn verify_inserts(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{gpu_insert::Inserts, gpu_texture};
    let (pass, color) = gpu_texture::depth_stencil_pass(ctx, 32, 32).unwrap();
    let inserts = Inserts::new(ctx, 24).unwrap();
    gpu_texture::begin_pass(
        ctx,
        Some(pass),
        PassAction::clear_color(0.25, 0.125, 0.0625, 0.5),
    );
    inserts.destination_alpha(ctx, false, [32, 32]).unwrap();
    ctx.end_render_pass();
    let mut pixels = vec![0; 32 * 32 * 4];
    ctx.texture_read_pixels(color, &mut pixels);
    for (index, p) in pixels.as_chunks::<4>().0.iter().enumerate() {
        assert_eq!(
            p,
            &[64, 32, 16, 0],
            "insert pixel {},{}",
            index % 32,
            index / 32
        );
    }
    gpu_texture::begin_pass(
        ctx,
        Some(pass),
        PassAction::clear_color(0.25, 0.125, 0.0625, 0.5),
    );
    inserts.destination_alpha(ctx, true, [32, 32]).unwrap();
    ctx.end_render_pass();
    ctx.texture_read_pixels(color, &mut pixels);
    for p in pixels.as_chunks::<4>().0 {
        assert!(p[0].abs_diff(192) <= 1 && p[1].abs_diff(160) <= 1 && p[2].abs_diff(144) <= 1);
        assert_eq!(p[3], 128);
    }
    inserts.delete(ctx);
    ctx.delete_render_pass(pass);
    println!("Material11 destination-alpha clearing and specular accumulation read back correctly");
}

fn verify_targets(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{gpu_target::Targets, scene::overlay::Viewport};
    let mut targets = Targets::default();
    let full = Viewport {
        screen: 0,
        render_target: -1,
        window: [0, 0, 32, 32],
    };
    let color = targets.begin(ctx, full, [32, 32], 3, 0xffff0000).unwrap();
    ctx.end_render_pass();
    let crop = Viewport {
        window: [8, 4, 24, 12],
        ..full
    };
    assert_eq!(
        targets.begin(ctx, crop, [32, 32], 1, 0xff0000ff).unwrap(),
        color
    );
    ctx.end_render_pass();
    let mut pixels = vec![0; 32 * 32 * 4];
    ctx.texture_read_pixels(color, &mut pixels);
    for y in 0..32 {
        for x in 0..32 {
            let pixel = &pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
            assert_eq!(
                pixel,
                if (8..24).contains(&x) && (20..28).contains(&y) {
                    &[0, 0, 255, 255]
                } else {
                    &[255, 0, 0, 255]
                }
            );
        }
    }
    assert_eq!(targets.len(), 1);
    assert_eq!(targets.texture(0, -1), Some(color));
    let present = kkrieger_oxide::gpu_present::Present::new(ctx).unwrap();
    let (output, copied) = kkrieger_oxide::gpu_texture::depth_stencil_pass(ctx, 32, 32).unwrap();
    kkrieger_oxide::gpu_texture::begin_pass(ctx, Some(output), PassAction::default());
    // Leave a stale restricted scissor to catch incomplete final presentation.
    ctx.apply_scissor_rect(2, 2, 4, 4);
    present.draw(ctx, color);
    ctx.end_render_pass();
    let mut copied_pixels = vec![0; pixels.len()];
    ctx.texture_read_pixels(copied, &mut copied_pixels);
    assert_eq!(
        copied_pixels, pixels,
        "final presentation changed orientation or pixels"
    );
    present.delete(ctx);
    ctx.delete_render_pass(output);

    assert!(
        targets
            .begin(
                ctx,
                Viewport {
                    window: [-1, 0, 2, 2],
                    ..full
                },
                [32, 32],
                0,
                0
            )
            .is_err()
    );
    let resized = targets
        .begin(
            ctx,
            Viewport {
                window: [0, 0, 64, 32],
                ..full
            },
            [64, 32],
            3,
            0xff00ff00,
        )
        .unwrap();
    ctx.end_render_pass();
    assert_ne!(resized, color);
    assert_eq!(targets.len(), 1);
    targets.delete(ctx);
    println!(
        "Original target identities, cropped clears, and resized attachments read back correctly"
    );
}

fn verify_overlay_copy(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        gpu_overlay::Overlays,
        gpu_target::Targets,
        scene::{
            camera::Environment,
            overlay::{Draw, Viewport, fx_quad},
        },
    };
    let viewport = Viewport {
        screen: -1,
        render_target: 200,
        window: [0, 0, 32, 32],
    };
    let mut targets = Targets::default();
    let source = targets
        .begin(ctx, viewport, [32, 32], 3, 0xffff0000)
        .unwrap();
    ctx.end_render_pass();
    targets
        .begin(
            ctx,
            Viewport {
                window: [8, 4, 24, 12],
                ..viewport
            },
            [32, 32],
            1,
            0xff0000ff,
        )
        .unwrap();
    ctx.end_render_pass();
    let mut original = vec![0; 32 * 32 * 4];
    ctx.texture_read_pixels(source, &mut original);
    let destination = Viewport {
        render_target: 201,
        ..viewport
    };
    let output = targets.begin(ctx, destination, [32, 32], 3, 0).unwrap();
    let mut copies = Overlays::new(ctx, 24).unwrap();
    let mut draw = Draw {
        viewport: destination,
        environment: Environment {
            orthogonal: 1,
            ..Default::default()
        },
        material: 27,
        texture: 200,
        texture1: None,
        texture2: None,
        color: u32::MAX,
        color2: None,
        vertices: fx_quad(None, None, None, None),
    };
    copies.draw(ctx, &draw, source).unwrap();
    ctx.end_render_pass();
    let mut pixels = vec![0; original.len()];
    ctx.texture_read_pixels(output, &mut pixels);
    assert_eq!(pixels, original, "IPP copy changed framebuffer orientation");
    targets
        .begin(ctx, viewport, [32, 32], 3, 0x80402010)
        .unwrap();
    ctx.end_render_pass();
    draw.color = 0x8080ffff;
    let source_color = [
        64. / 255. * 128. / 255.,
        32. / 255.,
        16. / 255.,
        128. / 255. * 128. / 255.,
    ];
    let destination_color = [16. / 255., 32. / 255., 64. / 255., 128. / 255.];
    for mode in 0..6 {
        draw.material = 27 + mode;
        targets
            .begin(ctx, destination, [32, 32], 3, 0x80102040)
            .unwrap();
        copies.draw(ctx, &draw, source).unwrap();
        ctx.end_render_pass();
        ctx.texture_read_pixels(output, &mut pixels);
        for channel in 0..4 {
            let s = source_color[channel];
            let d = destination_color[channel];
            let expected: f32 = match mode {
                0 => s,
                1 => s + d,
                2 => s * d,
                3 => 2. * s * d,
                4 => s + d * (1. - s),
                _ => s * source_color[3] + d * (1. - source_color[3]),
            };
            let expected = (expected.clamp(0., 1.) * 255.).round() as u8;
            assert!(
                pixels[(16 * 32 + 16) * 4 + channel].abs_diff(expected) <= 2,
                "copy mode {mode} channel {channel}"
            );
        }
    }
    copies.delete(ctx);
    targets.delete(ctx);
    println!(
        "Original six IPP copy blends, color/alpha modulation, and target orientation read back correctly"
    );
}

fn verify_overlay_blur(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        gpu_overlay::Overlays,
        gpu_target::Targets,
        scene::{
            camera::Environment,
            overlay::{Draw, Viewport, fx_quad},
        },
    };
    let viewport = Viewport {
        screen: -1,
        render_target: 200,
        window: [0, 0, 32, 32],
    };
    let second = Viewport {
        render_target: 201,
        ..viewport
    };
    let destination = Viewport {
        render_target: 202,
        ..viewport
    };
    let mut targets = Targets::default();
    let source = targets
        .begin(ctx, viewport, [32, 32], 3, 0x80402010)
        .unwrap();
    ctx.end_render_pass();
    let source1 = targets.begin(ctx, second, [32, 32], 3, 0x40204060).unwrap();
    ctx.end_render_pass();
    let output = targets.begin(ctx, destination, [32, 32], 3, 0).unwrap();
    ctx.end_render_pass();
    let mut overlays = Overlays::new(ctx, 24).unwrap();
    let mut draw = Draw {
        viewport: destination,
        environment: Environment {
            orthogonal: 1,
            ..Default::default()
        },
        material: 4,
        texture: 200,
        texture1: Some(201),
        texture2: None,
        color: 0x40000000,
        color2: Some(u32::MAX),
        vertices: fx_quad(None, None, None, None),
    };
    let mut pixels = vec![0; 32 * 32 * 4];
    for pass in 0..2 {
        draw.material = 4 + pass;
        targets
            .begin(ctx, destination, [32, 32], if pass == 0 { 3 } else { 0 }, 0)
            .unwrap();
        overlays
            .draw_sources(ctx, &draw, &[source, source1])
            .unwrap();
        ctx.end_render_pass();
        ctx.texture_read_pixels(output, &mut pixels);
        for (channel, sum) in [96., 96., 112., 192.].into_iter().enumerate() {
            let expected: f32 = sum * (64. / 255.) * (pass + 1) as f32;
            assert!(
                pixels[(16 * 32 + 16) * 4 + channel].abs_diff(expected.round() as u8) <= 2,
                "blur pass {pass} channel {channel}"
            );
        }
    }
    // Independent UV1 must sample the vertically mirrored source rather than
    // reusing UV0. This also covers the two bindings referencing one target.
    targets
        .begin(ctx, viewport, [32, 32], 3, 0xffff0000)
        .unwrap();
    ctx.end_render_pass();
    targets
        .begin(
            ctx,
            Viewport {
                window: [8, 4, 24, 12],
                ..viewport
            },
            [32, 32],
            1,
            0xff0000ff,
        )
        .unwrap();
    ctx.end_render_pass();
    let mut original = vec![0; pixels.len()];
    ctx.texture_read_pixels(source, &mut original);
    draw.material = 4;
    draw.texture1 = Some(200);
    draw.vertices = fx_quad(None, None, None, Some([0., 1., 1., 0.]));
    targets.begin(ctx, destination, [32, 32], 3, 0).unwrap();
    overlays
        .draw_sources(ctx, &draw, &[source, source])
        .unwrap();
    ctx.end_render_pass();
    ctx.texture_read_pixels(output, &mut pixels);
    for y in 0..32 {
        for x in 0..32 {
            for channel in 0..4 {
                let index = (y * 32 + x) * 4 + channel;
                let mirrored = ((31 - y) * 32 + x) * 4 + channel;
                let expected = ((original[index] as f32 + original[mirrored] as f32) * (64. / 255.))
                    .round() as u8;
                assert!(
                    pixels[index].abs_diff(expected) <= 2,
                    "independent blur UV1 at {x},{y} channel {channel}"
                );
            }
        }
    }
    overlays.delete(ctx);
    targets.delete(ctx);
    println!(
        "Original two-pass blur weights, alpha accumulation, and independent UV sets read back correctly"
    );
}

fn verify_overlay_color_merge(ctx: &mut dyn RenderingBackend) {
    use kkrieger_oxide::{
        gpu_overlay::Overlays,
        gpu_target::Targets,
        scene::{
            camera::Environment,
            overlay::{Draw, Viewport, fx_quad},
        },
    };
    let viewport = Viewport {
        screen: -1,
        render_target: 200,
        window: [0, 0, 32, 32],
    };
    let second = Viewport {
        render_target: 201,
        ..viewport
    };
    let destination = Viewport {
        render_target: 202,
        ..viewport
    };
    let mut targets = Targets::default();
    let source = targets
        .begin(ctx, viewport, [32, 32], 3, 0x80406080)
        .unwrap();
    ctx.end_render_pass();
    let source1 = targets.begin(ctx, second, [32, 32], 3, 0x40802040).unwrap();
    ctx.end_render_pass();
    let output = targets.begin(ctx, destination, [32, 32], 3, 0).unwrap();
    ctx.end_render_pass();
    let mut overlays = Overlays::new(ctx, 24).unwrap();
    let mut draw = Draw {
        viewport: destination,
        environment: Environment {
            orthogonal: 1,
            ..Default::default()
        },
        material: 8,
        texture: 200,
        texture1: None,
        texture2: None,
        color: 0x80204060,
        color2: Some(0x00404040),
        vertices: fx_quad(None, None, None, None),
    };
    fn combine(a: [f32; 3], b: [f32; 3], operation: usize, alpha: f32) -> [f32; 3] {
        std::array::from_fn(|i| match operation {
            0 => a[i] + b[i],
            1 => a[i] * (1. - b[i]) + b[i],
            2 => b[i] - a[i],
            3 => a[i] - b[i],
            4 => a[i] * b[i],
            5 => a[i] * b[i] * 2.,
            6 => a[i] * alpha + b[i] * (1. - alpha),
            _ => a[0] * b[0] + a[1] * b[1] + a[2] * b[2],
        })
    }
    let mut pixels = vec![0; 32 * 32 * 4];
    for material in 8..=24 {
        draw.material = material;
        let (a, b, operation, alpha) = if material < 16 {
            draw.texture1 = None;
            (
                [32. / 255., 64. / 255., 96. / 255.],
                [64. / 255., 96. / 255., 128. / 255.],
                material - 8,
                128. / 255.,
            )
        } else {
            draw.texture1 = Some(201);
            draw.color = 0x80000000;
            // Native AlphaSrc construction masks to the low nibble. Thus
            // MERGE8's authored INVERTA flag is ignored, just as in source.
            (
                [64. / 255., 96. / 255., 128. / 255.],
                [128. / 255., 32. / 255., 64. / 255.],
                if material == 24 { 6 } else { material - 16 },
                if material == 24 {
                    64. / 255.
                } else {
                    128. / 255.
                },
            )
        };
        let expected =
            combine(a, b, operation, alpha).map(|v| (v * (4. * 64. / 255.)).clamp(0., 1.));
        targets.begin(ctx, destination, [32, 32], 3, 0).unwrap();
        if material < 16 {
            overlays.draw(ctx, &draw, source).unwrap();
        } else {
            overlays
                .draw_sources(ctx, &draw, &[source, source1])
                .unwrap();
        }
        ctx.end_render_pass();
        ctx.texture_read_pixels(output, &mut pixels);
        for (channel, value) in expected.into_iter().enumerate() {
            let wanted = (value * 255.).round() as u8;
            assert!(
                pixels[(16 * 32 + 16) * 4 + channel].abs_diff(wanted) <= 2,
                "overlay material {material} channel {channel}: got {} expected {wanted}",
                pixels[(16 * 32 + 16) * 4 + channel]
            );
        }
        assert_eq!(pixels[(16 * 32 + 16) * 4 + 3], 255);
    }
    overlays.delete(ctx);
    targets.delete(ctx);
    println!("Original eight color operations and nine merge materials read back correctly");
}
