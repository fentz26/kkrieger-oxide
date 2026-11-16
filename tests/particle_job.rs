use kkrieger_oxide::{
    kx::Document,
    mesh::identity,
    scene::{exec::Execution, particles::Particle},
};
fn document() -> Document<'static> {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.operators.truncate(2);
    let scene = d.classes.iter().position(|c| c.id == 0xc1).unwrap();
    let effect = d.classes.iter().position(|c| c.id == 0x64).unwrap();
    for op in &mut d.operators {
        op.inputs.clear();
        op.links.clear();
        op.animation = &[1];
    }
    d.operators[0].class = scene;
    d.operators[0].inputs = vec![1];
    d.operators[1].class = effect;
    d.operators[1].animation = &[2, 5, 0x91, 5, 1];
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
    d
}
#[test]
fn deferred_particle_job_evaluates_captured_variables_then_draws_and_ages() {
    let d = document();
    let mut e = Execution::new(&d);
    e.vm.variables[5] = [4.; 4];
    e.frame(&d, 0).unwrap();
    let job = e.jobs[0].clone();
    e.particles[2].particles.push(Particle {
        position: [0.; 4],
        speed: [1., 0., 0., 0.],
        time: 0,
        max_time: 100,
    });
    e.vm.variables[5] = [9.; 4];
    e.time_delta = 10;
    let geometry = e.draw_particle_job(&d, &job, identity(), Some(1)).unwrap();
    assert_eq!(geometry.len(), 1);
    assert_eq!(&geometry[0][0][..3], &[4., 0., 0.]);
    assert_eq!(e.particles[2].particles[0].time, 10);
    assert_eq!(e.particles[2].particles[0].position[0], 10.);
    assert_eq!(e.vm.variables[5], [4.; 4]);
    assert!(
        e.draw_particle_job(&d, &job, identity(), None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(e.particles[2].particles[0].time, 10);
}

#[test]
fn deferred_particle_handler_restores_local_variables_when_slot_is_invalid() {
    let mut d = document();
    d.operators[1].parameters[0] = 16;
    d.operators[1].animation = &[24, 0, 0, 128, 63, 0x81, 7, 1];
    let mut e = Execution::new(&d);
    e.vm.variables[7] = [8.; 4];
    e.frame(&d, 0).unwrap();
    let job = e.jobs[0].clone();
    let error = e
        .draw_particle_job(&d, &job, identity(), Some(1))
        .unwrap_err();
    assert_eq!(error.reason, "particle slot out of range");
    assert_eq!(e.vm.variables[7], [8.; 4]);
}

#[test]
fn particle_render_pass_resolves_prepared_material_and_uses_camera_axes() {
    use kkrieger_oxide::{
        material,
        scene::{camera::Environment, exec::Job, overlay::Viewport, viewport::RenderPass},
    };
    use std::{collections::BTreeMap, rc::Rc};
    let source = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = source
        .operators
        .iter()
        .position(|op| source.classes[op.class].id == 0xd0)
        .unwrap();
    let generated = material::generate_many(&source, &[root]).unwrap();
    let mut prepared = (*generated[0]).clone();
    assert!(!prepared.passes.is_empty());
    prepared.passes[0].render_pass = 12;
    let mut d = document();
    let mut material_op = source.operators.into_iter().nth(root).unwrap();
    material_op.inputs.clear();
    material_op.links.clear();
    material_op.animation = &[1];
    d.operators.push(material_op);
    d.operators[1].links = vec![Some(2)];
    let mut e = Execution::new(&d);
    e.vm.variables[5] = [4.; 4];
    e.frame(&d, 0).unwrap();
    let mut job = e.jobs[0].clone();
    if let Job::Paint { pass_adjust, .. } = &mut job {
        *pass_adjust = 3;
    }
    e.particles[2].particles.push(Particle {
        position: [0.; 4],
        speed: [0.; 4],
        time: 0,
        max_time: 100,
    });
    let camera = Environment {
        space: [
            [0., 1., 0., 0.],
            [-1., 0., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
        ..Environment::default()
    };
    let pass = RenderPass {
        viewport: Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        camera,
        clear_flags: 0,
        clear_color: 0,
        paint_mode: 0,
        jobs: vec![job],
        grab_stereo_target: false,
        blend_stereo_target: false,
        allocate_stereo_target: false,
    };
    let cache = BTreeMap::from([(2, Rc::new(prepared))]);
    e.time_delta = 10;
    let packets = e.draw_particle_pass(&d, &pass, &cache).unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].operator, 1);
    assert_eq!(packets[0].sort_key, 0x0f070000);
    assert_eq!(packets[0].material, Some(2));
    assert_eq!(&packets[0].quads[0][0][..3], &[0., 4., 0.]);
    assert_eq!(e.particles[2].particles[0].time, 10);
    assert!(e.draw_particle_pass(&d, &pass, &BTreeMap::new()).is_err());
    assert_eq!(e.particles[2].particles[0].time, 10);
}
