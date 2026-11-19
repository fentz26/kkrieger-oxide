use kkrieger_oxide::{
    game::{
        Inventory,
        beta_monster::{
            HitBox, ai,
            frame::{ActorSimulation, Settings},
            physics,
            registry::Registry,
            static_world::StaticWorld,
        },
    },
    mesh::identity,
};
#[test]
fn actor_frame_runs_physics_then_ai_with_captured_scene_colliders() {
    let world = StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]);
    let mut simulation = ActorSimulation::new(world);
    let mut registry = Registry::default();
    let mut data = [0; 26];
    data[6] = 1f32.to_bits();
    data[7] = 10f32.to_bits();
    data[8] = 10f32.to_bits();
    data[10] = 20;
    data[12] = 10;
    data[13] = 0.01f32.to_bits();
    data[15] = 0.01f32.to_bits();
    data[20..23].fill(1f32.to_bits());
    let token = registry.spawn(&data, identity(), 0, |_| Some(0)).unwrap();
    registry.register(token).unwrap();
    let mut inventory = Inventory::default();
    inventory.amounts[0] = 100;
    let settings = Settings {
        physics: physics::Settings {
            gravity: 0.,
            gravity_player: 0.,
            damping_player_air: 0.,
            damping_player_ground: 0.,
            fly_mode: 0,
            on_ground: false,
        },
        ai: ai::Settings {
            gravity: 0.,
            height: 1.,
            height_gain: 0.,
        },
        player: [-1., 0., 0., 1.],
        player_cell: Some(0),
        first_particle: 100,
    };
    let accepted = simulation.physics(&registry, &[], &settings).unwrap();
    assert_eq!(inventory.amounts[0], 100);
    assert!(!registry.get(token).unwrap().borrow().active);
    let sounds = simulation
        .think(&registry, &accepted, &mut inventory, &settings)
        .unwrap();
    assert_eq!(sounds.len(), 1);
    assert_eq!(inventory.amounts[0], 90);
    let actor = registry.get(token).unwrap();
    assert!(actor.borrow().active);
    assert_ne!(actor.borrow().hit_cell.mode & 0x20, 0);
    let sounds = simulation
        .step(&registry, &[token], &mut inventory, &settings)
        .unwrap();
    assert_eq!(sounds.len(), 1);
    assert_eq!(inventory.amounts[0], 80);
    assert_eq!(actor.borrow().hit_cell.mode & 0x20, 0);
    assert_eq!(simulation.hits.shots.len(), 5);
    assert!(
        actor
            .borrow()
            .body
            .particles
            .iter()
            .all(|particle| particle.cell == Some(0))
    );
    actor.borrow_mut().initial.life = 0;
    assert!(
        simulation
            .step(&registry, &[], &mut inventory, &settings)
            .unwrap()
            .is_empty()
    );
    assert_eq!(inventory.amounts[0], 80);
}

#[test]
fn owned_frame_advances_player_body_in_shared_physics() {
    use kkrieger_oxide::game::beta_monster::{body::Body, runtime::BodyRegistration};
    let world = StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]);
    let mut simulation = ActorSimulation::new(world);
    let mut registry = Registry::default();
    let mut data = [0; 26];
    data[10] = 100;
    let token = registry.spawn(&data, identity(), 0, |_| Some(0)).unwrap();
    registry.register(token).unwrap();
    registry.get(token).unwrap().borrow_mut().active = true;
    let mut prefix = [(
        Body::new([2., 0., 0., 1.], Some(0), 1),
        BodyRegistration {
            particles: [0, 1, 2, 3, 4],
            skip_cell: None,
        },
    )];
    let mut settings = Settings {
        physics: physics::Settings {
            gravity: 0.,
            gravity_player: 0.01,
            damping_player_air: 0.,
            damping_player_ground: 0.,
            fly_mode: 0,
            on_ground: false,
        },
        ai: ai::Settings {
            gravity: 0.01,
            height: 1.,
            height_gain: 0.,
        },
        player: [2., 0., 0., 1.],
        player_cell: Some(0),
        first_particle: 0,
    };
    let before = prefix[0].0.clone();
    assert!(
        simulation
            .physics_with_bodies(&registry, &[token], &mut prefix, &settings)
            .is_err()
    );
    assert_eq!(prefix[0].0, before);
    assert!(simulation.hits.shots.is_empty());
    settings.first_particle = 5;
    simulation
        .physics_with_bodies(&registry, &[token], &mut prefix, &settings)
        .unwrap();
    assert!(prefix[0].0.particles[0].vectors[0][1] < 0.);
    assert_eq!(simulation.hits.shots.len(), 10);
    assert_eq!(simulation.hits.shots[&0].kind, 1);
    assert_eq!(simulation.hits.shots[&5].kind, 3);
    assert!(registry.get(token).unwrap().borrow().body.particles[0].vectors[0][1] < 0.5);
}

#[test]
fn released_player_clearance_uses_native_downward_world_cast() {
    let world = StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [2.; 3],
        }
        .vertices(),
        0,
    )]);
    let mut simulation = ActorSimulation::new(world);
    let base = simulation
        .player_clearance([0., 0., 0., 1.], None, 0.01, 1.)
        .unwrap();
    assert!(base > 1. && base < 1.011);
    let clearance = simulation
        .player_clearance([0., -0.5, 0., 1.], Some(0), 0.01, 1.)
        .unwrap();
    assert!((clearance - 0.505).abs() < 0.00001); // Released hull includes 0.005 overlap.
    assert!(
        simulation
            .player_clearance([0., 0., 0., 1.], Some(99), 0.01, 1.)
            .is_err()
    );
}

#[test]
fn released_player_particles_collide_with_scene_owned_wall() {
    use kkrieger_oxide::{
        collision::{Cell, DynamicCell},
        game::beta_monster::{body::Body, runtime::BodyRegistration, scene_cells::SceneCell},
    };
    use std::{cell::RefCell, rc::Rc};
    let world = StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]);
    let mut simulation = ActorSimulation::new(world);
    let registry = Registry::default();
    let mut matrix = identity();
    matrix[3][0] = 1.5;
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [1., 4., 4.], 1),
        matrix,
    )));
    source.borrow_mut().cell.mode &= !0x20;
    let scene = SceneCell::capture(Rc::clone(&source), false).unwrap();
    let mut body = Body::new([0., 0., 0., 1.], Some(0), 1);
    body.translate([2.5, 0., 0., 0.]);
    let mut prefix = [(
        body,
        BodyRegistration {
            particles: [0, 1, 2, 3, 4],
            skip_cell: None,
        },
    )];
    let settings = Settings {
        physics: physics::Settings {
            gravity: 0.,
            gravity_player: 0.,
            damping_player_air: 0.,
            damping_player_ground: 0.,
            fly_mode: 0,
            on_ground: false,
        },
        ai: ai::Settings {
            gravity: 0.,
            height: 1.,
            height_gain: 0.,
        },
        player: [0., 0., 0., 1.],
        player_cell: Some(0),
        first_particle: 5,
    };
    let accepted = simulation
        .physics_with_scene(&registry, &[], &mut prefix, &settings, vec![scene])
        .unwrap();
    assert!(prefix[0].0.particles[0].vectors[0][0] < 1.);
    assert_eq!(accepted.dynamic_views().unwrap().len(), 1);
    assert_eq!(source.borrow().cell.mode & 0x20, 0);
}

#[test]
fn frame_reconciliation_includes_registered_point_projectiles() {
    use kkrieger_oxide::{
        collision::{Cell, DynamicCell},
        game::beta_monster::{
            hit::Shot,
            projectile::{PhysicsSlot, Point},
            registration::ActorFrame,
            runtime::{CollisionRuntime, PhysicsRoster, RegisteredPoint},
            scene_cells::SceneCell,
            subtractive::CandidateWorld,
        },
    };
    use std::{cell::RefCell, rc::Rc};
    let mut simulation = ActorSimulation::new(StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]));
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [2.; 3], 1),
        identity(),
    )));
    source.borrow_mut().cell.mode &= !0x20;
    let registry = Registry::default();
    let mut accepted = ActorFrame::capture(&registry, &[], 5).unwrap();
    accepted
        .attach_scene(vec![SceneCell::capture(Rc::clone(&source), false).unwrap()])
        .unwrap();
    accepted.scene[0].cell.current[3][0] = 0.2;
    let captured = accepted.clone();
    let mut views = captured.dynamic_views().unwrap();
    let mut words = [0; 12];
    words[10] = 1f32.to_bits();
    words[11] = 8;
    let point = Rc::new(RefCell::new(
        Point::new(&words, identity(), Some(0), None).unwrap(),
    ));
    simulation.hits.shots.insert(
        999,
        Shot {
            kind: 8,
            event: None,
        },
    );
    let points = [RegisteredPoint {
        point: Rc::clone(&point),
        particle: 999,
        skip_cell: None,
    }];
    let registrations = Default::default();
    let order = [PhysicsSlot::Point(0)];
    let roster = PhysicsRoster {
        registry: &registry,
        colliders: &[],
        registrations: &registrations,
        points: &points,
        boxes: &[],
        order: &order,
    };
    let adds = simulation.world.additive_views();
    let statics = simulation.world.static_views();
    let lists: Vec<_> = simulation
        .world
        .subtractive_lists
        .iter()
        .map(Vec::as_slice)
        .collect();
    let zone_lists: Vec<_> = simulation
        .world
        .zone_lists
        .iter()
        .map(Vec::as_slice)
        .collect();
    let mut runtime = CollisionRuntime {
        adds: &adds,
        candidates: CandidateWorld {
            static_cells: &statics,
            lists: &lists,
            dynamic_cells: &mut views,
        },
        zones: &mut [],
        zone_lists: &zone_lists,
        hits: &mut simulation.hits,
        particle_id: 999,
        skip_cell: None,
        monster_mode: false,
        interrupted: false,
        error: None,
    };
    let before = point.borrow().particle.vectors[0];
    accepted
        .reconcile_all_with_roster(&mut runtime, roster, &mut [])
        .unwrap();
    assert!(accepted.scene[0].cell.pose.blocked);
    assert_eq!(source.borrow().blocked, 1);
    assert_eq!(point.borrow().particle.vectors[0], before);
}
