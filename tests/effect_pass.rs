use kkrieger_oxide::{
    bitmap::{FontPage, Letter},
    kx::Document,
    material::{self, Usage},
    mesh::identity,
    scene::{
        camera::Environment,
        effects::Content,
        exec::{Execution, Job},
        overlay::Viewport,
        particles::Particle,
        viewport::RenderPass,
    },
};
use std::{collections::BTreeMap, rc::Rc};
#[test]
fn mixed_print_and_particle_pass_preserves_original_equal_key_order() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d
        .operators
        .iter()
        .position(|o| d.classes[o.class].id == 0xd0)
        .unwrap();
    let generated = material::generate_many(&d, &[root]).unwrap();
    let mut material = (*generated[0]).clone();
    material.passes[0].render_pass = 3;
    material.passes[0].usage = Usage::Other;
    let print = d.classes.iter().position(|c| c.id == 0x61).unwrap();
    let particle = d.classes.iter().position(|c| c.id == 0x64).unwrap();
    for op in &mut d.operators[..2] {
        op.animation = &[1];
        op.inputs.clear();
        op.links = vec![Some(root)];
    }
    d.operators[0].class = print;
    d.operators[0].parameters = vec![4, 1f32.to_bits(), 1f32.to_bits(), 0, 0, 0];
    d.operators[0].strings = vec![b"X"];
    d.operators[1].class = particle;
    d.operators[1].parameters = vec![
        2,
        1,
        0xffffffff,
        0xffffffff,
        0xffffffff,
        1f32.to_bits(),
        1f32.to_bits(),
        1f32.to_bits(),
        0.5f32.to_bits(),
        0.1f32.to_bits(),
        0,
        0,
    ];
    let jobs = (0..2)
        .map(|operator| Job::Paint {
            operator,
            matrix: identity(),
            time: 0.,
            pass_adjust: 0,
            effect_variables: None,
        })
        .collect();
    let pass = RenderPass {
        viewport: Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        camera: Environment::default(),
        clear_flags: 0,
        clear_color: 0,
        paint_mode: 0,
        jobs,
        grab_stereo_target: false,
        blend_stereo_target: false,
        allocate_stereo_target: false,
    };
    let materials = BTreeMap::from([(root, Rc::new(material))]);
    let pages = BTreeMap::from([(
        0,
        FontPage {
            page: 0,
            line_height: 16,
            letters: [Letter {
                uv: [0., 0., 1., 1.],
                width: 1.,
                ..Letter::default()
            }; 256],
        },
    )]);
    let mut e = Execution::new(&d);
    e.time_delta = 10;
    e.particles[2].particles.push(Particle {
        position: [0.; 4],
        speed: [0.; 4],
        time: 0,
        max_time: 100,
    });
    let draws = e
        .draw_effect_pass(&d, &pass, &materials, &pages, 60.)
        .unwrap();
    assert_eq!(draws.iter().map(|d| d.operator).collect::<Vec<_>>(), [1, 0]);
    assert!(matches!(&draws[0].content,Content::Particles(quads) if quads.len()==1));
    assert!(matches!(&draws[1].content,Content::Print(draw) if draw.geometry.vertices.len()==4));
    assert!(
        draws
            .iter()
            .all(|d| d.sort_key == 0x03070000 && d.material == Some(root))
    );
    assert_eq!(e.particles[2].particles[0].time, 10);
}
