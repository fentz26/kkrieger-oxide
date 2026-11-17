use kkrieger_oxide::{
    bitmap::{FontPage, Letter},
    kx::{Class, Document},
    material,
    scene::{
        camera::Environment,
        effects::Content,
        exec::{Execution, Job},
        overlay::Viewport,
        viewport::RenderPass,
    },
};
use std::collections::BTreeMap;
#[test]
fn deferred_load_effect_executes_wrapper_animation_and_preserves_snapshot() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.operators.truncate(4);
    let print = d.classes.iter().position(|c| c.id == 0x61).unwrap();
    let scene = d.classes.iter().position(|c| c.id == 0xc1).unwrap();
    let mat = d.classes.iter().position(|c| c.id == 0xd0).unwrap();
    let load = d.classes.len();
    d.classes.push(Class {
        id: 1,
        convention: 0,
        packing: &[],
    });
    for (op, class) in d.operators.iter_mut().zip([print, load, scene, mat]) {
        op.class = class;
        op.inputs.clear();
        op.links.clear();
        op.animation = &[1];
        op.parameters.clear();
    }
    d.operators[0].parameters = vec![4, 1f32.to_bits(), 1f32.to_bits(), 0, 0, 0];
    d.operators[0].strings = vec![b"A"];
    d.operators[0].links = vec![Some(3)];
    d.operators[0].animation = &[2, 5, 0x91, 1, 1];
    d.operators[1].links = vec![Some(0)];
    d.operators[1].animation = &[24, 0, 0, 0, 64, 0x8f, 5, 1];
    d.operators[2].inputs = vec![1];
    let mut parameters = vec![0; 64];
    parameters[..48].copy_from_slice(&material::State::default().words);
    parameters[9] = 0x10;
    parameters[48] = 8 << 20;
    parameters[49] = 7;
    d.operators[3].parameters = parameters;
    let materials = BTreeMap::from([(3, material::generate_many(&d, &[3]).unwrap().remove(0))]);
    let mut e = Execution::new(&d);
    e.vm.variables[5] = [3.; 4];
    e.frame(&d, 2).unwrap();
    assert_eq!(e.jobs.len(), 1);
    assert!(matches!(e.jobs[0], Job::Paint { operator: 1, .. }));
    e.vm.variables[5] = [9.; 4];
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
        jobs: e.jobs.clone(),
        grab_stereo_target: false,
        blend_stereo_target: false,
        allocate_stereo_target: false,
    };
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
    assert!(
        e.draw_effect_pass(&d, &pass, &materials, &BTreeMap::new(), 60.)
            .is_err()
    );
    assert_eq!(e.vm.variables[5], [3.; 4]);
    let draws = e
        .draw_effect_pass(&d, &pass, &materials, &pages, 60.)
        .unwrap();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].material, Some(3));
    assert_eq!(draws[0].sort_key, 0x07070000);
    let Content::Print(draw) = &draws[0].content else {
        panic!("expected Print")
    };
    assert_eq!(draw.geometry.vertices[1][0], 2.);
    assert_eq!(e.vm.variables[5], [3.; 4]);
    d.operators[0].class = d.classes.iter().position(|c| c.id == 0x64).unwrap();
    d.operators[0].animation = &[1];
    d.operators[1].animation = &[1];
    d.operators[0].parameters = vec![
        0,
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
    let mut particles = Execution::new(&d);
    particles.frame(&d, 2).unwrap();
    particles.particles[0]
        .particles
        .push(kkrieger_oxide::scene::particles::Particle {
            position: [0.; 4],
            speed: [1., 0., 0., 0.],
            time: 0,
            max_time: 100,
        });
    particles.time_delta = 10;
    let particle_pass = RenderPass {
        jobs: particles.jobs.clone(),
        ..pass
    };
    let packets = particles
        .draw_particle_pass(&d, &particle_pass, &materials)
        .unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].quads.len(), 1);
    assert_eq!(particles.particles[0].particles[0].time, 10);
}
