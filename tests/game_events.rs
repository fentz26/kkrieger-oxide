use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        event_runtime::{Runtime, Shot},
        player::Motion,
        shot, zones,
    },
    kx::Document,
    mesh::identity,
    random::Random,
    scene::{event::Event, exec::Execution},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn operator(&mut self) -> Option<usize> {
        if self.word() != 0 {
            Some(self.word() as usize)
        } else {
            None
        }
    }
}
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
fn document() -> Document<'static> {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc
}
fn origin() -> shot::Origin {
    shot::Origin::Player {
        position: [0., 0., 0., 1.],
        look: 0.,
        direction: 0.,
        last_camera_y: 0.,
        camera_offset: 0.,
        cell: 0,
    }
}
#[test]
fn original_game_event_registration_order_weapon_payload_and_shot_removal() {
    let doc = document();
    let world = world();
    let mut r = Reader(include_bytes!("fixtures/game-event-registration.bin"));
    assert_eq!(r.word(), 96);
    assert_eq!(r.word(), 4);
    for case in 0..96 {
        let mut execution = Execution::new(&doc);
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        let mut runtime = Runtime {
            zones: Vec::new(),
            weapon: 12,
            shots: Vec::new(),
        };
        let mut rng = Random::new(1);
        assert_eq!(r.word(), 6);
        for id in 0..6 {
            let operator = Some(r.word() as usize);
            let select = r.int();
            let mode = r.int();
            let marker = r.word();
            let event = execution.events.insert(
                Event {
                    operator,
                    select,
                    ..Event::default()
                },
                false,
            );
            assert_eq!(event, id);
            let mut state = shot::spawn(0, operator, origin(), &mut rng)
                .unwrap()
                .unwrap();
            state.mode = mode;
            state.matrix = identity();
            state.matrix[3][0] = marker as f32;
            runtime.shots.push(Shot { state, event });
        }
        for i in 0..3 {
            let mut ids = [None; 2];
            for (slot, value) in ids.iter_mut().enumerate() {
                let operator = r.operator();
                let id = execution.events.insert(
                    Event {
                        operator,
                        ..Event::default()
                    },
                    false,
                );
                assert_eq!(id, 6 + 2 * i + slot as u64);
                if operator.is_some() {
                    *value = Some(id);
                }
            }
            runtime.zones.push((i as usize, ids));
        }
        let operator = r.operator();
        assert_eq!(
            execution.events.insert(
                Event {
                    operator,
                    ..Event::default()
                },
                false
            ),
            12
        );
        execution.game.weapon.current = r.word() as usize;
        execution.game.weapon.next = r.word() as usize;
        for i in 0..8 {
            execution.game.events.optics[i] = r.operator();
            execution.game.parameters.weapon_cool[i] = r.float();
        }
        let mut has_instance = true;
        for tick in 0..4 {
            execution.game.weapon.timer = r.float();
            execution.game.weapon.cooldown = r.float();
            motion.collider.position = r.vector();
            motion.last_camera_y = r.float();
            execution.game.parameters.camera_offset = r.float();
            motion.look = r.float();
            motion.direction = r.float();
            let zones = zones::Runtime {
                respawn: (0..3).map(|_| r.int()).collect(),
            };
            let old = execution.game.weapon.current;
            execution.begin_events(0, false);
            runtime.add_events(&mut execution, &motion, &zones).unwrap();
            let changed = old != execution.game.weapon.current;
            assert_eq!(r.word(), u32::from(changed && has_instance));
            if changed {
                has_instance = false;
            }
            assert_eq!(execution.game.weapon.current, r.word() as usize);
            assert_eq!(execution.game.weapon.timer == 0., r.word() != 0);
            assert_eq!(execution.game.weapon.timer.to_bits(), r.word());
            assert_eq!(runtime.shots.len(), r.word() as usize);
            for shot in &runtime.shots {
                assert_eq!(shot.event, u64::from(r.word()));
            }
            for op in 0..3 {
                let count = r.word();
                let ids: Vec<_> = (0..count).map(|_| u64::from(r.word())).collect();
                assert_eq!(execution.events.active(op), ids, "{case}/{tick}");
            }
            for id in 0..6 {
                assert_eq!(
                    execution.events.events[&id].borrow().matrix[3][0],
                    r.word() as f32
                );
            }
            let event = execution.events.events[&runtime.weapon].borrow();
            assert_eq!(event.operator, r.operator());
            for v in [event.translate, event.rotate] {
                for x in v {
                    assert_eq!(x.to_bits(), r.word());
                }
            }
            assert_eq!(event.velocity.to_bits(), r.word());
            assert_eq!(event.modulation.to_bits(), r.word());
            assert_eq!(has_instance, r.word() != 0);
            drop(event);
            execution.finish_events();
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn projectile_lifecycle_queues_explosion_and_releases_shot_event() {
    let doc = document();
    let mut world = world();
    let mut execution = Execution::new(&doc);
    let motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
    execution.game.events.shot[0] = Some(0);
    execution.game.events.explode[0][0] = Some(1);
    let mut runtime = Runtime::new(&world, &mut execution);
    let mut rng = Random::new(1);
    runtime.fire(&mut execution, 0, origin(), &mut rng).unwrap();
    assert_eq!(runtime.shots.len(), 1);
    let event = runtime.shots[0].event;
    execution.begin_events(0, false);
    assert!(execution.events.active(0).is_empty());
    runtime
        .add_events(&mut execution, &motion, &zones::Runtime::new(&world))
        .unwrap();
    assert_eq!(execution.events.active(0), [event]);
    execution.finish_events();
    for _ in 0..500 {
        runtime
            .advance_shots(&mut execution, &motion, &mut world)
            .unwrap();
        if runtime.shots.is_empty() {
            break;
        }
    }
    assert!(runtime.shots.is_empty());
    assert!(!execution.events.events.contains_key(&event));
    assert_eq!(execution.events.dynamic.len(), 1);
    let explosion = execution.events.dynamic[0];
    assert_eq!(
        execution.events.events[&explosion].borrow().operator,
        Some(1)
    );
    assert!(execution.events.active(1).is_empty());
    execution.begin_events(100, false);
    assert_eq!(execution.events.active(1), [explosion]);
}

fn event_document() -> Document<'static> {
    use kkrieger_oxide::kx::Class;
    let mut doc = document();
    doc.classes = vec![
        Class {
            id: 6,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0x14,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![1f32.to_bits()];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![10, 2f32.to_bits(), 1f32.to_bits()];
    doc
}
#[test]
fn respawn_and_weapon_switch_clear_their_persistent_scene_instances() {
    use kkrieger_oxide::collision::world::EventSpawn;
    let doc = event_document();
    let mut world = world();
    world.entries.push(Entry {
        cell: Cell::new(identity(), [1.; 3], 2),
        sector: None,
        events: std::array::from_fn(|_| {
            Some(EventSpawn {
                operator: 0,
                translate: [2., 3., 4., 0.],
                cull_distance: 35.,
            })
        }),
        logic: Logic {
            code: 0,
            condition: 0,
            value: 0,
            output: 0,
            events: [Some(0), Some(0)],
        },
    });
    world.zones.push(1);
    let mut execution = Execution::new(&doc);
    let mut runtime = Runtime::new(&world, &mut execution);
    let mut zones = zones::Runtime::new(&world);
    let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
    for id in runtime.zones[0].1.into_iter().flatten() {
        assert_eq!(
            execution.events.events[&id].borrow().translate,
            [2., 3., 4., 0.]
        );
        assert_eq!(execution.events.events[&id].borrow().cull_distance, 35);
    }
    zones.respawn[0] = 100;
    for expected in [true, false] {
        execution.splash.enabled = false;
        execution.begin_events(0, false);
        runtime.add_events(&mut execution, &motion, &zones).unwrap();
        execution.frame(&doc, 0).unwrap();
        assert_eq!(execution.splash.enabled, expected);
        execution.finish_events();
    }
    zones.respawn[0] = 5;
    let resets = zones
        .tick(
            &mut execution.game,
            &mut motion,
            &mut world,
            &execution.dynamic_cells,
            &mut execution.switches,
        )
        .unwrap();
    assert_eq!(resets, [1]);
    runtime.reset_zone_events(&mut execution, &resets).unwrap();
    zones.respawn[0] = 100;
    execution.splash.enabled = false;
    execution.begin_events(0, false);
    runtime.add_events(&mut execution, &motion, &zones).unwrap();
    execution.frame(&doc, 0).unwrap();
    assert!(execution.splash.enabled);
    execution.finish_events();
    runtime
        .restart(&mut execution, &mut motion, &world, &mut zones)
        .unwrap();
    assert_eq!(zones.respawn, [0]);
    zones.respawn[0] = 100;
    execution.splash.enabled = false;
    execution.begin_events(0, false);
    runtime.add_events(&mut execution, &motion, &zones).unwrap();
    execution.frame(&doc, 0).unwrap();
    assert!(execution.splash.enabled);
    execution.finish_events();
    // Remove zone registration so only the persistent weapon event can trigger.
    runtime.zones.clear();
    zones.respawn.clear();
    execution.game.events.optics[0] = Some(0);
    execution.game.events.optics[1] = Some(0);
    for (next, timer, expected) in [(0, 1., true), (0, 0., false), (1, 1., true)] {
        execution.game.weapon.next = next;
        execution.game.weapon.timer = timer;
        execution.splash.enabled = false;
        execution.begin_events(0, false);
        runtime.add_events(&mut execution, &motion, &zones).unwrap();
        execution.frame(&doc, 0).unwrap();
        assert_eq!(execution.splash.enabled, expected);
        execution.finish_events();
    }
}
