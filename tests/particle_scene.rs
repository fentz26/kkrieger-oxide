use kkrieger_oxide::{
    kx::{Class, Document},
    scene::exec::Execution,
};
fn document() -> Document<'static> {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.classes = [0x63, 0x64, 0xc1, 0xffff]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    d.operators.truncate(4);
    for (i, op) in d.operators.iter_mut().enumerate() {
        op.class = i;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters.clear();
    }
    let mut p = vec![0; 17];
    p[0] = 2;
    p[2] = 3;
    p[3] = 1f32.to_bits();
    p[4] = 1000f32.to_bits();
    d.operators[0].parameters = p;
    d.operators[0].inputs = vec![3];
    d.operators[1].parameters = vec![2, 20];
    d.operators[2].inputs = vec![0, 1];
    d
}
#[test]
fn emitter_scene_handler_does_not_execute_its_inputs() {
    let d = document();
    let mut e = Execution::new(&d);
    e.frame(&d, 0).unwrap();
    assert!(e.jobs.is_empty());
}

#[test]
fn prepared_particle_slots_share_particles_but_instances_retain_separate_bursts() {
    use kkrieger_oxide::{
        random::Random,
        scene::{cache_scene_identity, event::Event},
    };
    let mut d = document();
    d.operators[0].inputs.clear();
    let mut e = Execution::new(&d);
    e.prepare_particle_systems(&d, 2).unwrap();
    assert_eq!(e.particles[2].capacity, 20);
    assert_eq!(cache_scene_identity(&d, 0).unwrap(), Some(0));
    let mut random = Random::new(7);
    e.current_time = 2;
    e.time_delta = 2;
    e.frame_with_random(&d, 2, &mut random).unwrap();
    assert_eq!(e.particles[2].particles.len(), 2);
    assert!(e.jobs.iter().any(|job| matches!(
        job,
        kkrieger_oxide::scene::exec::Job::Paint { operator: 1, .. }
    )));
    e.current_time = 4;
    e.frame_with_random(&d, 2, &mut random).unwrap();
    assert_eq!(e.particles[2].particles.len(), 3);
    e.current_time = 6;
    e.frame_with_random(&d, 2, &mut random).unwrap();
    assert_eq!(e.particles[2].particles.len(), 3);
    // A new event owns a new emitter burst, sharing the same global slot.
    let id = e.events.insert(Event::default(), false);
    e.execute_event(&d, 0, id, kkrieger_oxide::mesh::identity())
        .unwrap();
    assert_eq!(e.particles[2].particles.len(), 5);
    e.reset_view();
    e.current_time = 8;
    e.time_delta = 2;
    e.frame_with_random(&d, 2, &mut random).unwrap();
    assert_eq!(e.particles[2].particles.len(), 7);
    e.prepare_particle_systems(&d, 2).unwrap();
    assert!(e.particles[2].particles.is_empty());
}
#[test]
fn particle_precalc_preserves_slots_not_reached_by_the_selected_root() {
    let d = document();
    let mut e = Execution::new(&d);
    e.particles[5].capacity = 99;
    e.prepare_particle_systems(&d, 2).unwrap();
    assert_eq!(e.particles[5].capacity, 99);
    assert_eq!(e.particles[2].capacity, 20);
}

#[test]
fn emitter_advances_the_callers_random_stream_even_when_later_execution_fails() {
    use kkrieger_oxide::{
        mesh::identity,
        random::Random,
        scene::particles::{Emitter, Parameters, System},
    };
    let mut d = document();
    d.operators[0].inputs.clear();
    d.operators[2].inputs = vec![0, 3];
    let mut e = Execution::new(&d);
    e.prepare_particle_systems(&d, 1).unwrap();
    e.current_time = 2;
    e.time_delta = 2;
    let mut actual = Random::new(7);
    assert!(e.frame_with_random(&d, 2, &mut actual).is_err());
    let mut expected = Random::new(7);
    let mut system = System {
        particles: vec![],
        capacity: 20,
    };
    Emitter::default().emit(
        &mut system,
        &Parameters {
            mode: 0,
            burst: 3,
            lifetime: 1.,
            rate: 1000.,
            speed: [0.; 4],
            random_position: [0.; 4],
            random_speed: [0.; 4],
        },
        identity(),
        2,
        2,
        &mut expected,
    );
    assert_eq!(actual.next_u32(), expected.next_u32());
    assert_eq!(e.particles[2].particles.len(), 2);
}
