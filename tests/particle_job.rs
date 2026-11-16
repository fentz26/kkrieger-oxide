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
