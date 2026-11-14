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
