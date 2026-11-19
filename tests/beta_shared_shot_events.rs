use kkrieger_oxide::game::beta_monster::{
    hit::{HitRegistry, Shot},
    subtractive::{DynamicEffects, ParticleContact},
};
#[test]
fn particles_sharing_one_scene_event_observe_hit_state_immediately() {
    let mut registry = HitRegistry::default();
    let mut words = [0; 45];
    words[2] = 50;
    words[3] = 100;
    for id in [10, 11] {
        registry.shots.insert(
            id,
            Shot {
                kind: 8,
                event: Some(words),
            },
        );
        registry.bind_event(id, 7, words).unwrap();
    }
    registry.operators[0][0] = 19;
    registry.time = 20;
    let hit = ParticleContact {
        cell: 0,
        particle: 10,
        kind: 8,
        position: [0., 0., 0., 1.],
        plane: [0., 1., 0., 0.],
        monster: false,
    };
    let mut select = registry.first_particle(10).unwrap().shot_select;
    registry.particle_hit(hit, &mut select);
    assert_eq!(registry.error, None);
    assert_eq!(registry.pending.len(), 1);
    assert_eq!(registry.first_particle(11).unwrap().shot_select, Some(1));
    assert_eq!(registry.shots[&11].event.unwrap()[3], 1);
    let mut select = registry.first_particle(11).unwrap().shot_select;
    registry.particle_hit(
        ParticleContact {
            particle: 11,
            ..hit
        },
        &mut select,
    );
    assert_eq!(registry.pending.len(), 1);
    assert_eq!(registry.shots[&10].event, registry.shots[&11].event);
    registry.clear_event_bindings();
    words[12] = 0;
    words[2] = 70;
    registry.bind_event(10, 8, words).unwrap();
    assert_eq!(registry.first_particle(10).unwrap().shot_select, Some(0));
    registry.bind_event(11, 8, words).unwrap();
    registry.flags[8] = 2;
    registry.time = 40;
    let count = registry.pending.len();
    let mut select = registry.first_particle(10).unwrap().shot_select;
    registry.particle_hit(hit, &mut select);
    assert_eq!(registry.pending.len(), count + 1);
    assert_eq!(registry.shots[&11].event.unwrap()[5], 40);
    assert_eq!(registry.first_particle(11).unwrap().shot_select, Some(0));
    let mut select = registry.first_particle(11).unwrap().shot_select;
    registry.particle_hit(
        ParticleContact {
            particle: 11,
            ..hit
        },
        &mut select,
    );
    assert_eq!(
        registry.pending.len(),
        count + 1,
        "shared reflection time suppresses a second effect"
    );
}

#[test]
fn point_event_binding_and_commit_preserve_live_event_fields_and_retry() {
    use kkrieger_oxide::{
        game::beta_monster::projectile::{Point, sync_hit_events},
        mesh::identity,
        scene::event::{Event, Scheduler},
    };
    let mut events = Scheduler::default();
    let a = events.insert(
        Event {
            start: 50,
            end: 100,
            operator: Some(17),
            interval: [0.25, 0.75],
            ..Default::default()
        },
        true,
    );
    let b = events.insert(
        Event {
            start: 60,
            end: 110,
            ..Default::default()
        },
        true,
    );
    let mut data = [0; 12];
    data[0] = 0x20;
    data[10] = 1f32.to_bits();
    data[11] = 8;
    let point = Point::new(&data, identity(), Some(0), Some(a)).unwrap();
    let other = Point::new(&data, identity(), Some(0), Some(b)).unwrap();
    let mut hits = HitRegistry::default();
    let held = events.events[&a].borrow_mut();
    assert_eq!(
        point.register_hit_state(10, &events, &mut hits),
        Err("released projectile event already borrowed")
    );
    assert!(!hits.shots.contains_key(&10));
    drop(held);
    point.register_hit_state(10, &events, &mut hits).unwrap();
    other.register_hit_state(12, &events, &mut hits).unwrap();
    hits.operators[0][0] = 19;
    hits.time = 20;
    let mut select = hits.first_particle(10).unwrap().shot_select;
    hits.particle_hit(
        ParticleContact {
            cell: 0,
            particle: 10,
            kind: 8,
            position: [0., 0., 0., 1.],
            plane: [0., 1., 0., 0.],
            monster: false,
        },
        &mut select,
    );
    let held = events.events[&b].borrow_mut();
    assert_eq!(
        sync_hit_events(&events, &hits),
        Err("released projectile event already borrowed")
    );
    assert_eq!(events.events[&a].borrow().select, 0);
    drop(held);
    sync_hit_events(&events, &hits).unwrap();
    let event = events.events[&a].borrow();
    assert_eq!(
        (event.start, event.end, event.select, event.shot_time),
        (0, 1, 1, 20)
    );
    assert_eq!(event.operator, Some(17));
    assert_eq!(event.interval, [0.25, 0.75]);
    assert_eq!(event.modulation, 5.);
}
