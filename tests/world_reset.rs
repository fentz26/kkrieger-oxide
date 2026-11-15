use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, EventSpawn, Logic, World},
    },
    game::{shot::Origin, simulation::Simulation},
    kx::Document,
    mesh::identity,
    random::Random,
    scene::{event::Event, exec::Execution},
};
use std::rc::Rc;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn float(&mut self, n: f32) {
        assert_eq!(n.to_bits(), self.word());
    }
}
fn world(zones: usize) -> World {
    let mut w = World {
        entries: vec![],
        adds: vec![],
        subs: vec![],
        zones: vec![],
        connectivity: connectivity::connect(&[]),
        faces: vec![],
    };
    for i in 0..zones {
        w.zones.push(i);
        w.entries.push(Entry {
            cell: Cell::new(identity(), [1.; 3], 2),
            sector: None,
            logic: Logic {
                code: 0,
                condition: 0,
                value: 0,
                output: 0,
                events: [None, None],
            },
            events: std::array::from_fn(|slot| {
                (slot == 0 || i % 2 == 1).then_some(EventSpawn {
                    operator: 0,
                    translate: [0.; 4],
                    cull_distance: 10.,
                })
            }),
        });
        w.faces.push(None);
    }
    w
}
#[test]
fn original_flush_and_root_replacement_preserve_persistent_state() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut r = Reader(include_bytes!("fixtures/flush.bin"));
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let c = case as f32;
        let mut e = Execution::new_player(&doc);
        let mut sim = Simulation::new(
            world(case % 4),
            &mut e,
            [0., 0., 0., 1.],
            123,
            [1, 2],
            Random::new(1),
        )
        .unwrap();
        let weapon = sim.events.weapon;
        let weapon_handle = e.events.events[&weapon].clone();
        weapon_handle.borrow_mut().modulation = c * 0.25;
        let shared_world = Rc::clone(&sim.world);
        let hit = Rc::clone(&sim.motion.hit_cell);
        let old_zones: Vec<_> = sim
            .events
            .zones
            .iter()
            .flat_map(|(_, events)| events.iter().flatten().copied())
            .collect();
        e.game.events.shot.fill(Some(0));
        e.game.events.optics.fill(Some(1));
        e.game.events.explode.fill([Some(2); 8]);
        for _ in 0..case % 7 {
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
        }
        let old_shots: Vec<_> = sim.events.shots.iter().map(|shot| shot.event).collect();
        let unrelated = e.events.insert(Event::default(), true);
        e.game.weapon.timer = c + 0.5;
        sim.motion.step_timer = c + 0.5;
        sim.motion.step_foot = case as i32 + 9;
        sim.motion.off_ground_time = c + 0.5;
        e.game.weapon.current = 7;
        e.game.weapon.next = 6;
        e.game.inventory.amounts[0] = 20 + case as i32;
        e.game.weapon.fire_key = case % 2 != 0;
        e.game.weapon.use_key = case % 2 != 0;
        e.game.weapon.cooldown = c * 0.125;
        sim.motion.accel_forward = c * 0.0625;
        sim.motion.speed_up = c * -0.125;
        sim.tick_count = 55;
        e.switches[1] = 6;
        e.switches[8] = 2;
        e.switches[40] = 1;
        e.game.parameters.start_position = [1., 2., 3., 1.];
        sim.reset_root(world(0), &mut e, [4, 5], case % 2 != 0)
            .unwrap();
        for n in [
            sim.world.borrow().adds.len(),
            sim.world.borrow().subs.len(),
            sim.world.borrow().zones.len(),
            sim.events.shots.len(),
        ] {
            assert_eq!(n, r.word() as usize);
        }
        assert_eq!(r.word(), 0);
        assert_eq!(r.word(), 0); // Native particle/constraint counters.
        r.float(e.game.weapon.timer);
        r.float(sim.motion.step_timer);
        r.float(sim.motion.off_ground_time);
        assert_eq!(sim.motion.step_foot, r.word() as i32);
        assert_eq!(e.game.weapon.current, r.word() as usize);
        assert_eq!(e.game.weapon.next, r.word() as usize);
        for i in 0..8 {
            for slot in [
                e.game.events.shot[i],
                e.game.events.optics[i],
                e.game.events.explode[0][i],
                e.game.events.explode[1][i],
            ] {
                assert_eq!(u32::from(slot.is_some()), r.word());
            }
        }
        for n in [
            e.game.inventory.amounts[0],
            i32::from(e.game.weapon.fire_key),
            i32::from(e.game.weapon.use_key),
        ] {
            assert_eq!(n, r.word() as i32);
        }
        r.float(e.game.weapon.cooldown);
        r.float(weapon_handle.borrow().modulation);
        r.float(sim.motion.accel_forward);
        r.float(sim.motion.speed_up);
        for n in [
            3 * (case % 5),
            case % 5,
            case % 5 + case % 3 + case % 4,
            case % 4 + (case % 4) / 2,
            case % 7,
        ] {
            assert_eq!(n, r.word() as usize);
        }
        assert!(
            old_zones
                .iter()
                .chain(&old_shots)
                .all(|id| !e.events.events.contains_key(id))
        );
        assert!(e.events.events.contains_key(&unrelated));
        assert_eq!(sim.events.weapon, weapon);
        assert!(Rc::ptr_eq(&weapon_handle, &e.events.events[&weapon]));
        assert!(Rc::ptr_eq(&shared_world, &sim.world));
        assert!(Rc::ptr_eq(&hit, &sim.motion.hit_cell));
        assert_eq!(sim.tick_count, 55);
        assert_eq!(sim.input.last_time, 123);
        assert_eq!(sim.input.last_mouse, [4, 5]);
        assert_eq!(e.switches[1], if case % 2 != 0 { 8 } else { 6 });
        assert_eq!(e.switches[8], 2);
        assert_eq!(e.switches[40], 0);
        assert_eq!(sim.motion.collider.position, [1., 2., 3., 1.]);
        assert!(sim.player_cell.is_none());
    }
    assert!(r.0.is_empty());
}
#[test]
fn new_world_rebinds_zone_events_and_shared_actor_context() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut e = Execution::new_player(&doc);
    let mut sim = Simulation::new(
        world(1),
        &mut e,
        [0., 0., 0., 1.],
        0,
        [0, 0],
        Random::new(1),
    )
    .unwrap();
    sim.bind_scene(&mut e, false);
    let old = e.actors.as_ref().unwrap().world.clone();
    let mut next = world(3);
    let cell = Cell::new(identity(), [20.; 3], 0);
    next.adds.push(3);
    next.entries.push(Entry {
        cell,
        sector: None,
        logic: Logic {
            code: 0,
            condition: 0,
            value: 0,
            output: 0,
            events: [None, None],
        },
        events: [None, None],
    });
    next.faces.push(Some(vec![]));
    next.connectivity = connectivity::connect(
        &next
            .entries
            .iter()
            .map(|entry| entry.cell.clone())
            .collect::<Vec<_>>(),
    );
    e.game.parameters.start_position = [0., 0., 0., 1.];
    sim.reset_root(next, &mut e, [0, 0], false).unwrap();
    assert_eq!(sim.player_cell, Some(3));
    assert!(Rc::ptr_eq(&old, &sim.world));
    assert_eq!(old.borrow().zones.len(), 3);
    assert_eq!(sim.zones.respawn.len(), 3);
    assert_eq!(sim.events.zones.len(), 3);
    for (entry, events) in &sim.events.zones {
        for (slot, id) in events.iter().enumerate() {
            assert_eq!(id.is_some(), slot == 0 || entry % 2 == 1);
            if let Some(id) = id {
                assert_eq!(e.events.events[id].borrow().operator, Some(0));
            }
        }
    }
    assert_eq!(
        e.actors.as_ref().unwrap().player_position,
        sim.motion.collider.position
    );
}
