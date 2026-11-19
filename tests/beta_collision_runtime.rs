use kkrieger_oxide::game::beta_monster::{
    body::Body,
    collision::Completion,
    hit::{HitRegistry, Shot},
    runtime::CollisionRuntime,
    subtractive::{CandidateWorld, DynamicCell, StaticCell},
    world_cast::AdditiveCell,
    zones::Zone,
};
#[test]
fn adapter_casts_updates_zones_and_preserves_history_on_clear_move() {
    let planes = [
        [1., 0., 0., 2.],
        [-1., 0., 0., 2.],
        [0., 1., 0., 2.],
        [0., -1., 0., 2.],
        [0., 0., 1., 2.],
        [0., 0., -1., 2.],
    ];
    let zone_planes = [
        [1., 0., 0., 0.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let adds = [AdditiveCell {
        planes: &planes,
        neighbors: &[],
        only_for_shot: false,
    }];
    let statics: [StaticCell; 0] = [];
    let lists: [&[usize]; 1] = [&[]];
    let zone_lists: [&[usize]; 1] = [&[0]];
    let mut zones = [Zone {
        planes: &zone_planes,
        enter: 0,
        leave: 0,
        collided: 0,
    }];
    let mut hits = HitRegistry::default();
    hits.shots.insert(
        7,
        Shot {
            kind: 3,
            event: None,
        },
    );
    let mut dynamic = [DynamicCell {
        planes: &planes,
        mode: 1,
        only_for_shot: false,
        monster: false,
        player: false,
        part_box: None,
        collided: 0,
    }];
    let mut runtime = CollisionRuntime {
        adds: &adds,
        candidates: CandidateWorld {
            static_cells: &statics,
            lists: &lists,
            dynamic_cells: &mut dynamic,
        },
        zones: &mut zones,
        zone_lists: &zone_lists,
        hits: &mut hits,
        particle_id: 7,
        skip_cell: Some(0),
        monster_mode: false,
        interrupted: false,
        error: None,
    };
    let mut body = Body::new([-0.5, 0., 0., 1.], Some(0), 3);
    body.particles[0].vectors[0] = [0.5, 0., 0., 1.];
    let old = body.particles[0].vectors[2];
    assert_eq!(
        runtime.collide(&mut body.particles[0], false).unwrap(),
        Completion::Clear
    );
    assert_eq!(body.particles[0].vectors[1], [0.5, 0., 0., 1.]);
    assert_eq!(body.particles[0].vectors[2], old);
    assert_eq!(runtime.zones[0].enter, 8);
    assert_eq!(runtime.zones[0].collided, 8);
    assert!(runtime.error.is_none());
    assert_eq!(
        runtime.candidates.dynamic_cells[0].mode, 1,
        "owned cell restored after response"
    );
    assert_eq!(
        runtime.candidates.dynamic_cells[0].collided, 0,
        "owned cell was excluded"
    );
    let before = body.particles[0].clone();
    runtime.skip_cell = Some(99);
    assert_eq!(
        runtime.collide(&mut body.particles[0], false),
        Err("skip cell out of range")
    );
    assert_eq!(body.particles[0], before);
    runtime.skip_cell = Some(0);
    runtime.hits.shots.get_mut(&7).unwrap().kind = 8;
    assert_eq!(
        runtime.collide(&mut body.particles[0], false),
        Err("particle kind differs from registry")
    );
    assert_eq!(body.particles[0], before);

    let mut event = [0; 45];
    event[2] = 1;
    runtime.hits.shots.get_mut(&7).unwrap().event = Some(event);
    runtime.hits.operators[0][0] = 123;
    runtime.hits.time = 100;
    body.particles[0].flags[3] = 8;
    body.particles[0].vectors[1] = [-0.5, 0., 0., 1.];
    body.particles[0].vectors[0] = [3., 0., 0., 1.];
    assert_eq!(
        runtime.collide(&mut body.particles[0], false).unwrap(),
        Completion::Contact
    );
    assert!(body.particles[0].vectors[0][0] < 2.);
    assert_eq!(runtime.hits.pending.len(), 1);
    assert_eq!(runtime.hits.pending[0][0], 123);
    assert_eq!(runtime.hits.shots[&7].event.unwrap()[12], 1);
    assert_eq!(
        runtime.zones[0].enter, 8,
        "stopped shot suppresses zone masks"
    );
    assert_eq!(runtime.candidates.dynamic_cells[0].mode, 1);

    runtime.hits.shots.insert(
        7,
        Shot {
            kind: 3,
            event: None,
        },
    );
    body = Body::new([-0.5, 0., 0., 1.], None, 3);
    let settings = kkrieger_oxide::game::beta_monster::physics::Settings {
        gravity: 0.02,
        gravity_player: 0.01,
        damping_player_air: 0.,
        damping_player_ground: 0.,
        fly_mode: 0,
        on_ground: false,
    };
    runtime.step_particle(&mut body, 0, settings).unwrap();
    assert_eq!(
        body.particles[0].cell,
        Some(0),
        "reacquired containing hull"
    );
    assert_eq!(body.particles[0].flags[5], 0);
    assert!(body.particles[0].vectors[0][1] < -0.0099 && body.particles[0].vectors[0][1] > -0.0101);
    assert_eq!(body.particles[0].vectors[2][1].to_bits(), 0);
    assert_eq!(body.particles[0].vectors[0][1].to_bits(), 0xbc23d70a);
    let before = body.clone();
    runtime.particle_id = 99;
    assert_eq!(
        runtime.step_particle(&mut body, 0, settings),
        Err("shot identity missing")
    );
    assert_eq!(body, before, "registry errors fail before physics mutation");

    use kkrieger_oxide::game::beta_monster::{
        dynamic::{MovingCell, Pose},
        runtime::BodyRegistration,
    };
    runtime.particle_id = 99;
    runtime.skip_cell = None;
    for (i, p) in body.particles.iter_mut().enumerate() {
        runtime.hits.shots.insert(
            i,
            Shot {
                kind: p.flags[3],
                event: None,
            },
        );
        p.cell = Some(0);
        p.vectors[0][3] = 1.;
    }
    let first = kkrieger_oxide::mesh::identity();
    let mut current = first;
    current[3][0] = 0.2;
    let mut moving = MovingCell {
        original: planes,
        count: 6,
        confirmed: first,
        current,
        mode: 1,
        monster: false,
        pose: Pose::apply(&planes, first).unwrap(),
    };
    let before = std::array::from_fn::<_, 5, _>(|i| body.particles[i].vectors[0]);
    runtime
        .reconcile_cell(
            0,
            &mut moving,
            std::slice::from_mut(&mut body),
            &[BodyRegistration {
                particles: [0, 1, 2, 3, 4],
                skip_cell: None,
            }],
        )
        .unwrap();
    assert!(
        moving.pose.blocked,
        "real collision rejects particles already inside the registered hull"
    );
    for (p, position) in body.particles.iter().zip(before) {
        assert_eq!(p.vectors[0], position);
    }
    assert_eq!(runtime.particle_id, 99);
    assert_eq!(runtime.skip_cell, None);
    let player = Body::new([-0.5, 0., 0., 1.], Some(0), 1);
    for (id, particle) in player.particles.iter().enumerate() {
        runtime.hits.shots.insert(
            id,
            Shot {
                kind: particle.flags[3],
                event: None,
            },
        );
    }
    let mut prefix = [(
        player,
        BodyRegistration {
            particles: [0, 1, 2, 3, 4],
            skip_cell: None,
        },
    )];
    let registry = kkrieger_oxide::game::beta_monster::registry::Registry::default();
    moving.pose = Pose::apply(&planes, first).unwrap();
    runtime
        .reconcile_body_roster(0, &mut moving, &registry, &Default::default(), &mut prefix)
        .unwrap();
    assert!(
        moving.pose.blocked,
        "player participates in ordinary moving hull rejection"
    );
    assert_eq!(prefix[0].0.particles[0].vectors[0][0], -0.5);
    moving.pose = Pose::apply(&planes, first).unwrap();
    moving.monster = true;
    runtime
        .reconcile_body_roster(0, &mut moving, &registry, &Default::default(), &mut prefix)
        .unwrap();
    assert!(
        !moving.pose.blocked,
        "monster hulls still skip non-weapon player particles"
    );
    moving.monster = false;
    moving.confirmed = first;
    moving.pose = Pose::apply(&planes, first).unwrap();
    prefix[0].1.skip_cell = Some(0);
    runtime
        .reconcile_body_roster(0, &mut moving, &registry, &Default::default(), &mut prefix)
        .unwrap();
    assert!(!moving.pose.blocked, "player excludes its own dynamic cell");
    assert_eq!(runtime.particle_id, 99);
    assert_eq!(runtime.skip_cell, None);
}

#[test]
fn ai_world_casts_floor_and_visibility_against_additive_hull() {
    use kkrieger_oxide::game::beta_monster::{ai::World, runtime::AiWorld};
    let planes = [
        [1., 0., 0., 2.],
        [-1., 0., 0., 2.],
        [0., 1., 0., 2.],
        [0., -1., 0., 2.],
        [0., 0., 1., 2.],
        [0., 0., -1., 2.],
    ];
    let adds = [AdditiveCell {
        planes: &planes,
        neighbors: &[],
        only_for_shot: false,
    }];
    let lists: [&[usize]; 1] = [&[]];
    let mut hits = HitRegistry::default();
    let mut dynamic = [DynamicCell {
        planes: &planes,
        mode: 1,
        only_for_shot: false,
        monster: true,
        player: false,
        part_box: None,
        collided: 0,
    }];
    let mut runtime = CollisionRuntime {
        adds: &adds,
        candidates: CandidateWorld {
            static_cells: &[],
            lists: &lists,
            dynamic_cells: &mut dynamic,
        },
        zones: &mut [],
        zone_lists: &lists,
        hits: &mut hits,
        particle_id: 0,
        skip_cell: Some(0),
        monster_mode: false,
        interrupted: false,
        error: None,
    };
    let mut world = AiWorld {
        runtime: &mut runtime,
        player_cell: Some(0),
    };
    assert_eq!(world.find_cell([0., 0., 0., 1.]), Some(0));
    assert!(!world.occluded([0., 0., 0., 1.], [1., 0., 0., 1.]));
    assert!(world.occluded([0., 0., 0., 1.], [3., 0., 0., 1.]));
    assert_eq!(
        world.ground_distance([0., 0., 0., 1.], Some(0), 3.),
        Some(-1.)
    );
    assert!(world.runtime.error.is_none());
    assert_eq!(world.runtime.candidates.dynamic_cells[0].mode, 1);
    let mut data = [0; 26];
    data[6] = 1f32.to_bits();
    data[7] = 10f32.to_bits();
    data[8] = 10f32.to_bits();
    data[10] = 20;
    data[13] = 1f32.to_bits();
    data[15] = 1f32.to_bits();
    let mut actor = kkrieger_oxide::game::beta_monster::actor::Actor::spawn(
        &data,
        kkrieger_oxide::mesh::identity(),
        0,
        1,
        |position| world.find_cell(position),
    )
    .unwrap();
    actor.body.particles[0].vectors[0] = [0., 0., 0., 1.];
    actor.tick(
        [-1., 0., 0., 1.],
        &kkrieger_oxide::game::beta_monster::ai::Settings {
            gravity: 0.01,
            height: 3.,
            height_gain: 0.2,
        },
        &mut world,
    );
    assert!(actor.active);
    assert!(actor.body.particles[0].vectors[0][0] < 0.);
    assert!(actor.body.particles[0].vectors[0][1] > 0.2);
    assert_eq!(world.runtime.candidates.dynamic_cells[0].mode, 1);
    assert!(world.runtime.error.is_none());
    let settings = kkrieger_oxide::game::beta_monster::ai::Settings {
        gravity: 0.01,
        height: 3.,
        height_gain: 0.2,
    };
    let mut inventory = kkrieger_oxide::game::Inventory::default();
    inventory.amounts[0] = 100;
    actor.initial.copied.parameters[6] = 10;
    actor.cooldowns = [1.; 2];
    let sound = world
        .runtime
        .tick_actor(
            &mut actor,
            [-1., 0., 0., 1.],
            Some(0),
            &settings,
            &mut inventory,
        )
        .unwrap();
    assert_eq!(inventory.amounts[0], 90);
    assert!(sound.is_some());
    actor.initial.copied.parameters[8] = 5;
    actor.initial.attack_operator = 123;
    actor.cooldowns = [1.; 2];
    world
        .runtime
        .tick_actor(
            &mut actor,
            [-1.9, 0., 0., 1.],
            Some(0),
            &settings,
            &mut inventory,
        )
        .unwrap();
    assert_eq!(inventory.amounts[0], 90);
    assert_eq!(world.runtime.hits.pending.len(), 1);
    assert_eq!(world.runtime.hits.pending[0][0], 123);
    use kkrieger_oxide::scene::event::{ReleasedReference, Scheduler};
    let emitted = world.runtime.hits.pending[0];
    let mut scheduler = Scheduler::default();
    let events = scheduler
        .publish_released(&mut world.runtime.hits.pending, |kind, token| {
            match (kind, token) {
                (ReleasedReference::Operator, 123) => Ok(17),
                (ReleasedReference::Monster, 1) => Ok(0),
                _ => Err("unexpected event token"),
            }
        })
        .unwrap();
    assert!(world.runtime.hits.pending.is_empty());
    assert_eq!(events.len(), 1);
    let event = scheduler.events[&events[0]].borrow();
    assert_eq!(event.operator, Some(17));
    assert_eq!(event.monster, Some(0));
    assert_eq!(event.rotate.map(f32::to_bits), emitted[17..21]);
    assert_eq!(event.translate.map(f32::to_bits), emitted[21..25]);
    actor.cooldowns = [1.; 2];
    let mut actors = [(actor.clone(), Some(0)), (actor.clone(), Some(0))];
    world.runtime.skip_cell = None;
    let sounds = world
        .runtime
        .tick_actors(
            &mut actors,
            [-1., 0., 0., 1.],
            Some(0),
            &settings,
            &mut inventory,
        )
        .unwrap();
    assert_eq!(sounds.len(), 2);
    assert_eq!(inventory.amounts[0], 70);
    assert_eq!(world.runtime.skip_cell, None);
    actors[1].1 = Some(99);
    let before = actors[0].0.initial.event_words;
    assert_eq!(
        world
            .runtime
            .tick_actors(
                &mut actors,
                [-1., 0., 0., 1.],
                Some(0),
                &settings,
                &mut inventory
            )
            .unwrap_err(),
        "AI owned cell out of range"
    );
    assert_eq!(actors[0].0.initial.event_words, before);
    assert_eq!(inventory.amounts[0], 70);
    use kkrieger_oxide::game::beta_monster::{physics, runtime::BodyRegistration};
    let mut stored = [actor];
    for (id, particle) in stored[0].body.particles.iter_mut().enumerate() {
        particle.cell = Some(0);
        world.runtime.hits.shots.insert(
            id,
            Shot {
                kind: particle.flags[3],
                event: None,
            },
        );
    }
    let event_before = stored[0].initial.event_words;
    let mut moving_cell = stored[0].hit_cell.clone();
    let registrations = [BodyRegistration {
        particles: [0, 1, 2, 3, 4],
        skip_cell: Some(0),
    }];
    world
        .runtime
        .step_bodies(
            &mut stored,
            &registrations,
            physics::Settings {
                gravity: 0.,
                gravity_player: 0.,
                damping_player_air: 0.,
                damping_player_ground: 0.,
                fly_mode: 0,
                on_ground: false,
            },
            |runtime, actors| {
                assert_eq!(actors[0].initial.event_words, event_before);
                runtime.reconcile_cell(0, &mut moving_cell, actors, &registrations)?;
                actors[0].body.particles[0].force[0] = 0.01;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(stored[0].initial.event_words, event_before);
    assert_eq!(stored[0].body.particles[0].force, [0.; 4]);
    let mut registry = kkrieger_oxide::game::beta_monster::registry::Registry::default();
    let token = registry
        .spawn(&data, kkrieger_oxide::mesh::identity(), 0, |position| {
            world.find_cell(position)
        })
        .unwrap();
    registry.register(token).unwrap();
    registry.get(token).unwrap().borrow_mut().active = true;
    let mut registered = std::collections::BTreeMap::new();
    registered.insert(
        token,
        BodyRegistration {
            particles: [0, 1, 2, 3, 4],
            skip_cell: Some(0),
        },
    );
    world
        .runtime
        .step_registered(
            &registry,
            &registered,
            physics::Settings {
                gravity: 0.,
                gravity_player: 0.,
                damping_player_air: 0.,
                damping_player_ground: 0.,
                fly_mode: 0,
                on_ground: false,
            },
            |runtime, registry| {
                runtime.reconcile_registered(0, &mut moving_cell, registry, &registered)?;
                registry.visit_active(|_, actor| {
                    actor.body.particles[0].force[0] = 0.01;
                    Ok(())
                })
            },
        )
        .unwrap();
    assert_eq!(
        registry.get(token).unwrap().borrow().body.particles[0].force,
        [0.; 4]
    );
    assert!(registry.get(token).unwrap().borrow().body.particles[0].vectors[0][0] > 0.);

    let mut captured = kkrieger_oxide::game::beta_monster::registration::ActorFrame::capture(
        &registry,
        &[token],
        0,
    )
    .unwrap();
    captured.reconcile_all(world.runtime, &registry).unwrap();
    assert_eq!(
        registry.get(token).unwrap().borrow().hit_cell.mode & 0x20,
        0
    );
    assert_eq!(captured.cells[0].1.mode & 0x20, 0);

    assert_eq!(world.runtime.skip_cell, None);
    let sounds = world
        .runtime
        .tick_registered(
            &registry,
            &registered,
            [-1., 0., 0., 1.],
            Some(0),
            &settings,
            &mut inventory,
        )
        .unwrap();
    assert!(sounds.is_empty());
    assert!(registry.get(token).unwrap().borrow().active);
    assert_eq!(world.runtime.skip_cell, None);
    let inactive = registry
        .spawn(&data, kkrieger_oxide::mesh::identity(), 0, |_| None)
        .unwrap();
    registry.register(inactive).unwrap();
    registry.get(inactive).unwrap().borrow_mut().body.particles[0].force[0] = 0.03;
    let inactive_body = registry.get(inactive).unwrap().borrow().body.clone();
    world
        .runtime
        .step_registered(
            &registry,
            &registered,
            physics::Settings {
                gravity: 0.,
                gravity_player: 0.,
                damping_player_air: 0.,
                damping_player_ground: 0.,
                fly_mode: 0,
                on_ground: false,
            },
            |_, _| Ok(()),
        )
        .unwrap();
    assert_eq!(registry.get(inactive).unwrap().borrow().body, inactive_body);
    registry.get(token).unwrap().borrow_mut().active = false;
    registry.get(token).unwrap().borrow_mut().body.particles[0].force[0] = 0.02;
    world
        .runtime
        .step_collider_roster(
            &registry,
            &[token],
            &registered,
            physics::Settings {
                gravity: 0.,
                gravity_player: 0.,
                damping_player_air: 0.,
                damping_player_ground: 0.,
                fly_mode: 0,
                on_ground: false,
            },
            |_, registry| {
                registry.get(inactive).unwrap().borrow_mut().active = true;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(
        registry.get(token).unwrap().borrow().body.particles[0].force,
        [0.; 4]
    );
    assert_eq!(registry.get(inactive).unwrap().borrow().body, inactive_body);
}

#[test]
fn player_and_registered_actors_share_physics_phase_boundaries() {
    use kkrieger_oxide::game::beta_monster::{
        physics, registry::Registry, runtime::BodyRegistration,
    };
    let planes = [
        [1., 0., 0., 100.],
        [-1., 0., 0., 100.],
        [0., 1., 0., 100.],
        [0., -1., 0., 100.],
        [0., 0., 1., 100.],
        [0., 0., -1., 100.],
    ];
    let adds = [AdditiveCell {
        planes: &planes,
        neighbors: &[],
        only_for_shot: false,
    }];
    let lists: [&[usize]; 1] = [&[]];
    let mut hits = HitRegistry::default();
    let mut prefix = vec![(
        Body::new([0., 0., 0., 1.], Some(0), 1),
        BodyRegistration {
            particles: [0, 1, 2, 3, 4],
            skip_cell: None,
        },
    )];
    let mut registry = Registry::default();
    let mut data = [0; 26];
    data[10] = 100;
    let mut matrix = kkrieger_oxide::mesh::identity();
    matrix[3][0] = 2.;
    let token = registry.spawn(&data, matrix, 0, |_| Some(0)).unwrap();
    registry.register(token).unwrap();
    registry.get(token).unwrap().borrow_mut().active = true;
    for (id, particle) in prefix[0]
        .0
        .particles
        .iter()
        .chain(registry.get(token).unwrap().borrow().body.particles.iter())
        .enumerate()
    {
        hits.shots.insert(
            id,
            Shot {
                kind: particle.flags[3],
                event: None,
            },
        );
    }
    let registrations = std::collections::BTreeMap::from([(
        token,
        BodyRegistration {
            particles: [5, 6, 7, 8, 9],
            skip_cell: None,
        },
    )]);
    let mut runtime = CollisionRuntime {
        adds: &adds,
        candidates: CandidateWorld {
            static_cells: &[],
            lists: &lists,
            dynamic_cells: &mut [],
        },
        zones: &mut [],
        zone_lists: &lists,
        hits: &mut hits,
        particle_id: 999,
        skip_cell: None,
        monster_mode: false,
        interrupted: false,
        error: None,
    };
    let settings = physics::Settings {
        gravity: 0.,
        gravity_player: 0.01,
        damping_player_air: 0.,
        damping_player_ground: 0.,
        fly_mode: 0,
        on_ground: false,
    };
    let before = prefix[0].0.clone();
    let actor_y = registry.get(token).unwrap().borrow().body.particles[0].vectors[0][1];
    assert_eq!(
        runtime.step_body_roster(
            &registry,
            &[token],
            &Default::default(),
            &mut prefix,
            settings,
            |_, _, _| Ok(())
        ),
        Err("released body registration missing")
    );
    assert_eq!(prefix[0].0, before);
    runtime
        .step_body_roster(
            &registry,
            &[token],
            &registrations,
            &mut prefix,
            settings,
            |runtime, registry, prefix| {
                assert!(prefix[0].0.particles[0].vectors[0][1] < 0.);
                assert!(
                    registry.get(token).unwrap().borrow().body.particles[0].vectors[0][1] < actor_y
                );
                assert_eq!(prefix[0].0.particles[0].force, [0.; 4]);
                prefix[0].0.particles[0].force[0] = 0.1;
                registry.get(token).unwrap().borrow_mut().body.particles[0].force[0] = 0.1;
                assert_eq!(runtime.particle_id, 9);
                Ok(())
            },
        )
        .unwrap();
    assert!(prefix[0].0.particles[0].vectors[0][0] > 0.);
    assert!(registry.get(token).unwrap().borrow().body.particles[0].vectors[0][0] > 2.);
    assert_eq!(prefix[0].0.particles[0].force, [0.; 4]);
    assert_eq!(
        registry.get(token).unwrap().borrow().body.particles[0].force,
        [0.; 4]
    );
    assert_eq!(runtime.particle_id, 999);
    assert_eq!(runtime.skip_cell, None);
}
