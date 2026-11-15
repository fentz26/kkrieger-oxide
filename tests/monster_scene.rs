use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    kx::{Class, Document},
    mesh::identity,
    scene::{
        event::Event,
        exec::{ActorContext, Execution},
    },
};
use std::{cell::RefCell, rc::Rc};
fn setup() -> (Document<'static>, Execution) {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0x11, 0x14, 0xc3, 0x15, 6]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(7);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.splines.clear();
    }
    let mut p = vec![0; 29];
    p[10] = 10;
    p[20] = 0.75f32.to_bits();
    p[21] = 1.5f32.to_bits();
    p[22] = 0.5f32.to_bits();
    p[26] = 4;
    doc.operators[0].class = 0;
    doc.operators[0].parameters = p.clone();
    doc.operators[0].inputs = vec![1, 2];
    for (i, hits) in [(1, 10), (2, 20)] {
        doc.operators[i].class = 1;
        doc.operators[i].parameters = vec![hits, 8f32.to_bits(), 1f32.to_bits()];
    }
    doc.operators[3].class = 2;
    doc.operators[3].parameters = [1f32, 1., 1., 0., 0., 0., 2., 0.75, 0.5]
        .map(f32::to_bits)
        .to_vec();
    doc.operators[3].inputs = vec![0];
    doc.operators[4].class = 3;
    doc.operators[4].parameters = vec![7, 9, 4];
    doc.operators[4].inputs = vec![3];
    doc.operators[5].class = 4;
    doc.operators[5].parameters = vec![1f32.to_bits()];
    doc.operators[5].inputs = vec![4];
    doc.operators[6].class = 0;
    doc.operators[6].parameters = p;
    doc.operators[6].inputs = vec![1, 2];
    let cell = Cell::new(identity(), [20.; 3], 0);
    let world = World {
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
    };
    let mut execution = Execution::new(&doc);
    execution.actors = Some(ActorContext {
        world: Rc::new(RefCell::new(world)),
        player_position: [-2., 0., 3., 1.],
        player_hit: None,
        time_reset: false,
    });
    (doc, execution)
}
#[test]
fn monster_scene_reuses_actor_shares_event_data_and_preserves_live_death_instances() {
    let (doc, mut execution) = setup();
    execution.vm.variables[0] = [9.; 4];
    execution.vm.variables[27] = [7., 8., 9., 10.];
    execution.frame(&doc, 4).unwrap();
    assert_eq!(execution.game.monsters.active_ids(), [0]);
    let actor = execution.game.monsters.get(0).unwrap();
    let hit = Rc::clone(&actor.borrow().hit_cell);
    let live = Rc::clone(&actor.borrow().event);
    let death = Rc::clone(&actor.borrow().death);
    assert_eq!(actor.borrow().health.kill_switch, 7);
    assert_eq!(actor.borrow().spawn_switch, 9);
    assert_eq!(actor.borrow().parameters.flags, 4);
    assert!(execution.next_monster_override.is_none());
    assert!(execution.dynamic_cells.is_empty());
    assert!(!execution.splash.enabled);
    assert!(Rc::ptr_eq(&live, &execution.events.events[&0]));
    assert!(Rc::ptr_eq(&death, &execution.events.events[&1]));
    assert_eq!(hit.borrow().matrix1, live.borrow().matrix);
    for (state, life, expected, hits) in [
        (3, 10, true, 10),
        (4, 10, false, 10),
        (4, 0, true, 20),
        (4, 0, false, 20),
        (0, 0, false, 20),
        (3, 10, true, 10),
    ] {
        actor.borrow_mut().health.state = state;
        actor.borrow_mut().health.life = life;
        if state >= 3 && life > 0 {
            live.borrow_mut().matrix = actor.borrow().matrix;
        }
        execution.splash.enabled = false;
        execution.frame(&doc, 4).unwrap();
        assert_eq!(execution.splash.enabled, expected);
        if expected {
            assert_eq!(execution.splash.hits, hits);
        }
        assert!(Rc::ptr_eq(&hit, &actor.borrow().hit_cell));
        assert!(Rc::ptr_eq(&actor, &execution.game.monsters.get(0).unwrap()));
        assert_eq!(execution.vm.variables[0], [9.; 4]);
        if actor.borrow().health.life > 0 {
            assert_eq!(execution.game.monsters.active_ids(), [0]);
        } else {
            assert!(execution.game.monsters.is_empty());
        }
        if life > 0 && state >= 3 {
            assert_eq!(execution.dynamic_cells.len(), 1);
            assert_eq!(execution.vm.variables[16..24], actor.borrow().walk.legs);
            assert_eq!(
                execution.vm.variables[27],
                [actor.borrow().walk.leg_time, 8., 9., 10.]
            );
        }
    }
}
#[test]
fn override_applies_to_only_the_first_initialized_instance() {
    let (mut doc, mut execution) = setup();
    doc.operators[4].inputs = vec![3, 3];
    execution.frame(&doc, 4).unwrap();
    assert_eq!(execution.game.monsters.active_ids(), [0, 1]);
    let a = execution.game.monsters.get(0).unwrap();
    let b = execution.game.monsters.get(1).unwrap();
    assert!(!Rc::ptr_eq(&a, &b));
    assert_eq!(a.borrow().health.kill_switch, 7);
    assert_eq!(b.borrow().health.kill_switch, 0);
    assert_eq!(a.borrow().parameters.flags, 4);
    assert_eq!(b.borrow().parameters.flags, 0);
    execution.frame(&doc, 4).unwrap();
    assert_eq!(execution.game.monsters.active_ids(), [0, 1]);
    assert!(Rc::ptr_eq(&a, &execution.game.monsters.get(0).unwrap()));
}
#[test]
fn expiring_parent_event_recursively_releases_nested_monsters_and_their_events() {
    let (mut doc, mut execution) = setup();
    doc.operators[0].inputs[0] = 6;
    let parent = execution.events.insert(
        Event {
            operator: Some(5),
            start: 0,
            end: 100,
            ..Event::default()
        },
        true,
    );
    execution.begin_events(0, false);
    execution.frame(&doc, 5).unwrap();
    execution.finish_events();
    let root = execution.game.monsters.get(0).unwrap();
    root.borrow_mut().health.state = 3;
    root.borrow().event.borrow_mut().matrix = root.borrow().matrix;
    execution.begin_events(10, false);
    execution.frame(&doc, 5).unwrap();
    execution.finish_events();
    assert_eq!(execution.game.monsters.active_ids(), [0, 1]);
    assert_eq!(execution.events.events.len(), 5);
    execution.begin_events(100, false);
    execution.frame(&doc, 5).unwrap();
    execution.finish_events();
    assert!(!execution.events.events.contains_key(&parent));
    assert!(execution.events.events.is_empty());
    assert!(execution.game.monsters.get(0).is_none());
    assert!(execution.game.monsters.get(1).is_none());
    assert!(execution.game.monsters.is_empty());
}

#[test]
fn view_reset_clears_dynamic_and_default_actors_but_retains_static_event_instances() {
    let (doc, mut e) = setup();
    let static_event = e.events.insert(Event::default(), false);
    let dynamic_event = e.events.insert(Event::default(), true);
    e.execute_event(&doc, 0, static_event, identity()).unwrap();
    let retained = e.game.monsters.active_ids()[0];
    e.execute_event(&doc, 0, dynamic_event, identity()).unwrap();
    let dynamic_actor = e.game.monsters.active_ids()[1];
    e.frame(&doc, 0).unwrap();
    let default_actor = e.game.monsters.active_ids()[0];
    assert!(e.has_event_instances(static_event));
    assert!(e.has_event_instances(dynamic_event));
    e.reset_view();
    assert!(e.game.monsters.is_empty());
    assert!(e.game.monsters.get(dynamic_actor).is_none());
    assert!(e.game.monsters.get(default_actor).is_none());
    assert!(e.game.monsters.get(retained).is_some());
    assert!(e.has_event_instances(static_event));
    assert!(!e.has_event_instances(dynamic_event));
    assert!(!e.events.events.contains_key(&dynamic_event));
    e.execute_event(&doc, 0, static_event, identity()).unwrap();
    assert_eq!(e.game.monsters.active_ids(), [retained]);
    e.frame(&doc, 0).unwrap();
    let replacement = e.game.monsters.active_ids()[0];
    assert_ne!(replacement, default_actor);
    assert_ne!(replacement, retained);
    // Repeated resets have no stale cursor or event ownership to revisit.
    e.reset_view();
    e.reset_view();
    assert!(e.game.monsters.get(replacement).is_none());
    assert!(e.game.monsters.get(retained).is_some());
}

#[test]
fn view_reset_retains_per_operator_scene_instances() {
    let (mut doc, _) = setup();
    doc.classes.push(Class {
        id: 9,
        convention: 0,
        packing: &[],
    });
    doc.operators[4].class = doc.classes.len() - 1;
    doc.operators[4].parameters = vec![0, 0];
    doc.operators[4].inputs = vec![0];
    let mut e = Execution::new(&doc);
    let (_, context) = setup();
    e.actors = context.actors;
    e.frame(&doc, 4).unwrap();
    let actor = e.game.monsters.active_ids()[0];
    let handle = e.game.monsters.get(actor).unwrap();
    e.reset_view();
    assert!(e.game.monsters.is_empty());
    e.frame(&doc, 4).unwrap();
    assert_eq!(e.game.monsters.active_ids(), [actor]);
    assert!(Rc::ptr_eq(&handle, &e.game.monsters.get(actor).unwrap()));
}

#[test]
fn root_replacement_exits_zone_and_shot_chains_but_keeps_weapon_chain() {
    use kkrieger_oxide::{
        collision::world::EventSpawn,
        game::{shot::Origin, simulation::Simulation},
        random::Random,
    };
    let (doc, mut e) = setup();
    let context = e.actors.take().unwrap();
    let mut world = Rc::try_unwrap(context.world).ok().unwrap().into_inner();
    world.zones.push(world.entries.len());
    world.entries.push(Entry {
        cell: Cell::new(identity(), [1.; 3], 2),
        sector: None,
        logic: Logic {
            code: 0,
            condition: 0,
            value: 0,
            output: 0,
            events: [None, None],
        },
        events: [
            Some(EventSpawn {
                operator: 0,
                translate: [0.; 4],
                cull_distance: 10.,
            }),
            None,
        ],
    });
    world.faces.push(None);
    let mut sim =
        Simulation::new(world, &mut e, [0., 0., 0., 1.], 0, [0, 0], Random::new(1)).unwrap();
    sim.bind_scene(&mut e, false);
    let weapon = sim.events.weapon;
    e.events.events[&weapon].borrow_mut().operator = Some(0);
    let zone = sim.events.zones[0].1[0].unwrap();
    e.game.events.shot[0] = Some(0);
    sim.events
        .fire(
            &mut e,
            0,
            Origin::Player {
                position: [0., 0., 0., 1.],
                look: 0.,
                direction: 0.,
                last_camera_y: 0.,
                camera_offset: 0.,
                cell: 0,
            },
            &mut sim.random,
        )
        .unwrap();
    let shot = sim.events.shots[0].event;
    for id in [weapon, zone, shot] {
        e.execute_event(&doc, 0, id, identity()).unwrap();
    }
    let owned = e.game.monsters.active_ids().to_vec();
    assert_eq!(owned.len(), 3);
    let weapon_actor = e.game.monsters.get(owned[0]).unwrap();
    let empty = World {
        entries: vec![],
        adds: vec![],
        subs: vec![],
        zones: vec![],
        connectivity: connectivity::connect(&[]),
        faces: vec![],
    };
    sim.reset_root(empty, &mut e, [0, 0], false).unwrap();
    assert!(e.has_event_instances(weapon));
    for id in [zone, shot] {
        assert!(!e.has_event_instances(id));
        assert!(!e.events.events.contains_key(&id));
    }
    assert!(Rc::ptr_eq(
        &weapon_actor,
        &e.game.monsters.get(owned[0]).unwrap()
    ));
    for id in &owned[1..] {
        assert!(e.game.monsters.get(*id).is_none());
    }
    assert!(sim.events.shots.is_empty());
    assert!(sim.events.zones.is_empty());
}
