use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        State,
        event_runtime::Runtime,
        monster_actor::Context,
        player::Motion,
        shot::{Damage, Origin, Outcome},
    },
    kx::Document,
    mesh::identity,
    random::Random,
    scene::exec::Execution,
};
use std::rc::Rc;
fn world() -> World {
    let cell = Cell::new(identity(), [20.; 3], 0);
    World {
        connectivity: connectivity::connect(std::slice::from_ref(&cell)),
        entries: vec![Entry {
            cell,
            sector: None,
            events: [None, None],
            logic: Logic {
                code: 0,
                condition: 0,
                value: 0,
                output: 0,
                events: [None, None],
            },
        }],
        adds: vec![0],
        subs: vec![],
        zones: vec![],
        faces: vec![Some(vec![])],
    }
}
#[test]
fn stable_actor_identity_survives_frame_reordering_and_rejects_removed_targets() {
    let mut game = State::default();
    let a = game.monsters.allocate().unwrap();
    let b = game.monsters.allocate().unwrap();
    let aid = a.borrow().id;
    let bid = b.borrow().id;
    for actor in [&a, &b] {
        actor.borrow_mut().health.life = 20;
        actor.borrow_mut().health.state = 3;
    }
    game.monsters.register(bid).unwrap();
    game.monsters.register(aid).unwrap();
    assert_eq!(game.monsters.active_ids(), [bid, aid]);
    let mut switches = [0; 256];
    game.apply_shot_outcome(
        Outcome {
            alive: false,
            damage: Some(Damage::Monster {
                monster: aid,
                hits: 7,
            }),
            explosion: None,
        },
        &mut switches,
    )
    .unwrap();
    assert_eq!(a.borrow().health.life, 13);
    assert_eq!(a.borrow().health.state, 4);
    assert_eq!(b.borrow().health.life, 20);
    assert!(Rc::ptr_eq(&a, &game.monsters.get(aid).unwrap()));
    game.monsters.begin_frame();
    assert!(game.monsters.is_empty());
    assert_eq!(game.monsters.get(aid).unwrap().borrow().health.life, 13);
    game.monsters.register(aid).unwrap();
    game.monsters.register(aid).unwrap();
    assert_eq!(game.monsters.len(), 2);
    assert!(game.monsters.remove(aid).is_some());
    assert!(game.monsters.is_empty());
    assert!(game.monsters.get(aid).is_none());
    let c = game.monsters.allocate().unwrap();
    let cid = c.borrow().id;
    assert!(cid > bid);
    assert_ne!(cid, aid);
    assert!(
        game.apply_shot_outcome(
            Outcome {
                alive: false,
                damage: Some(Damage::Monster {
                    monster: aid,
                    hits: 99
                }),
                explosion: None
            },
            &mut switches
        )
        .is_err()
    );
    assert_eq!(b.borrow().health.life, 20);
    assert_eq!(c.borrow().health.life, 0);
}
#[test]
fn actual_projectile_hits_the_actor_object_and_queues_its_explosion() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    let mut execution = Execution::new(&doc);
    let mut world = world();
    let unused = execution.game.monsters.allocate().unwrap();
    unused.borrow_mut().health.life = 99;
    let target = execution.game.monsters.allocate().unwrap();
    let id = target.borrow().id;
    let mut p = [0; 29];
    p[10] = 5;
    p[17] = 7;
    p[20] = 2f32.to_bits();
    p[21] = 2f32.to_bits();
    p[22] = 2f32.to_bits();
    p[26] = 4;
    let mut parent = identity();
    parent[3] = [0., 0., 3., 1.];
    target
        .borrow_mut()
        .initialize(
            &p,
            Context {
                world: &mut world,
                dynamic: &[],
                player_position: [0., 0., -2., 1.],
                parent,
                spline: None,
                time: 0,
                time_reset: false,
                flags: None,
            },
        )
        .unwrap();
    target.borrow_mut().health.state = 3;
    execution.game.monsters.register(id).unwrap();
    let hit = Rc::clone(&target.borrow().hit_cell);
    hit.borrow_mut().physics(1.);
    execution.dynamic_cells.push(hit);
    execution.game.parameters.weapon_hits[0] = 7;
    execution.game.events.shot[0] = Some(0);
    execution.game.events.explode[1][0] = Some(2);
    execution.switches[7] = 255;
    let mut runtime = Runtime::new(&world, &mut execution);
    let motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
    let mut rng = Random::new(1);
    runtime
        .fire(
            &mut execution,
            0,
            Origin::Player {
                position: [0., 0., 0., 1.],
                look: 0.,
                direction: 0.,
                last_camera_y: 0.,
                camera_offset: 0.,
                cell: 0,
            },
            &mut rng,
        )
        .unwrap();
    for _ in 0..30 {
        runtime
            .advance_shots(&mut execution, &motion, &mut world)
            .unwrap();
        if runtime.shots.is_empty() {
            break;
        }
    }
    assert!(runtime.shots.is_empty());
    assert_eq!(target.borrow().health.life, 0);
    assert_eq!(target.borrow().health.state, 4);
    assert_eq!(execution.switches[7], 0);
    assert_eq!(unused.borrow().health.life, 99);
    assert_eq!(execution.events.dynamic.len(), 1);
    let event = execution.events.dynamic[0];
    assert_eq!(execution.events.events[&event].operator, Some(2));
}
