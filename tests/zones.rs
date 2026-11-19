use kkrieger_oxide::{
    collision::{
        Cell, DynamicCell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{State, player::Motion, zones::Runtime},
    mesh::identity,
};
use std::{cell::RefCell, rc::Rc};
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
    fn interactions(&mut self) -> kkrieger_oxide::collision::Interactions {
        kkrieger_oxide::collision::Interactions {
            enter: self.word(),
            leave: self.word(),
            collided: self.word(),
        }
    }
}
#[test]
fn original_zone_logic_respawn_countdowns_and_interaction_cleanup() {
    let mut r = Reader(include_bytes!("fixtures/zones.bin"));
    assert_eq!(r.word(), 832);
    assert_eq!(r.word(), 4);
    for case in 0..832 {
        let mut game = State::default();
        let mut switches = std::array::from_fn(|_| r.word() as u8);
        game.weapon.use_key = r.word() != 0;
        game.weapon.next = r.word() as usize;
        let speed = r.float();
        for i in 0..16 {
            let amount = r.int();
            let max = r.int();
            if i < 8 {
                game.inventory.amounts[i] = amount;
                game.inventory.maxima[i] = max;
            } else {
                game.inventory.weapons[i - 8] = amount;
                assert_eq!(max, 1);
            }
        }
        let mut entries = vec![Entry {
            cell: Cell::new(identity(), [20.; 3], 0),
            sector: None,
            events: [None, None],
            logic: Logic {
                code: 0,
                condition: 0,
                value: 0,
                output: 0,
                events: [None, None],
            },
        }];
        assert_eq!(r.word(), 3);
        let mut timers = Vec::new();
        for _ in 0..3 {
            let logic = Logic {
                code: r.word() as u8,
                condition: r.word() as u8,
                output: r.word() as u8,
                value: r.int(),
                events: [None, None],
            };
            timers.push(r.int());
            entries.push(Entry {
                cell: Cell::new(identity(), [1.; 3], 2),
                sector: None,
                events: [None, None],
                logic,
            });
        }
        let cells: Vec<_> = entries.iter().map(|e| e.cell.clone()).collect();
        let mut world = World {
            entries,
            adds: vec![0],
            subs: vec![],
            zones: vec![1, 2, 3],
            connectivity: connectivity::connect(&cells),
            faces: vec![Some(vec![]), None, None, None],
        };
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        motion.speed_up = speed;
        let dynamic: Vec<_> = (0..2)
            .map(|_| {
                Rc::new(RefCell::new(DynamicCell::new(
                    Cell::new(identity(), [1.; 3], 2),
                    identity(),
                )))
            })
            .collect();
        let mut runtime = Runtime::new(&world);
        runtime.respawn = timers;
        let mut deleted = [0; 3];
        let mut instance = [true; 3];
        for tick in 0..4 {
            for &entry in &world.zones {
                world.entries[entry].cell.interactions = r.interactions();
            }
            for cell in &dynamic {
                cell.borrow_mut().cell.interactions = kkrieger_oxide::collision::Interactions {
                    enter: 2,
                    leave: 0xff00,
                    collided: 0xff02,
                };
            }
            let resets = runtime
                .tick(&mut game, &mut motion, &mut world, &dynamic, &mut switches)
                .unwrap();
            for entry in resets {
                let i = entry - 1;
                if instance[i] {
                    deleted[i] += 1;
                }
                instance[i] = false;
            }
            for value in switches {
                assert_eq!(u32::from(value), r.word(), "{case}/{tick}");
            }
            assert_eq!(game.weapon.next, r.word() as usize);
            assert_eq!(motion.speed_up.to_bits(), r.word());
            for value in game
                .inventory
                .amounts
                .into_iter()
                .chain(game.inventory.weapons)
            {
                assert_eq!(value, r.int(), "{case}/{tick}");
            }
            for i in 0..3 {
                assert_eq!(runtime.respawn[i], r.int(), "{case}/{tick}/{i}");
                assert_eq!(world.entries[i + 1].cell.interactions, r.interactions());
                assert_eq!(deleted[i], r.word());
                assert_eq!(instance[i], r.word() != 0);
            }
            for cell in &dynamic {
                assert_eq!(r.word(), 0);
                assert_eq!(cell.borrow().cell.interactions, r.interactions());
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn released_actor_masks_reach_zone_actions_once_and_preserve_player_masks() {
    use kkrieger_oxide::game::beta_monster::{frame::ActorSimulation, static_world::StaticWorld};
    let cells = vec![
        Cell::new(identity(), [20.; 3], 0),
        Cell::new(identity(), [2.; 3], 4),
    ];
    let mut released =
        ActorSimulation::new(StaticWorld::new(cells.iter().map(|c| (c.vertices, c.mode))));
    let mut world = World {
        connectivity: connectivity::connect(&cells),
        entries: cells
            .into_iter()
            .map(|cell| Entry {
                cell,
                sector: None,
                events: [None, None],
                logic: Logic {
                    code: 0x93,
                    condition: 0,
                    value: 0,
                    output: 7,
                    events: [None, None],
                },
            })
            .collect(),
        adds: vec![0],
        subs: vec![],
        zones: vec![1],
        faces: vec![None, None],
    };
    let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
    let mut game = State::default();
    let mut switches = [0; 256];
    switches[0] = 1;
    let mut runtime = Runtime::new(&world);
    world.entries[1].cell.interactions.collided = 2;
    released.zone_masks[1] = [0x100, 0x200, 0x400];
    released.publish_zone_interactions(&mut world).unwrap();
    assert_eq!(world.entries[1].cell.interactions.collided, 0x402);
    assert_eq!(world.entries[1].cell.interactions.leave, 0x200);
    assert!(released.zone_masks.iter().all(|m| *m == [0; 3]));
    runtime
        .tick(&mut game, &mut motion, &mut world, &[], &mut switches)
        .unwrap();
    assert_eq!(switches[7], 1);
    released.publish_zone_interactions(&mut world).unwrap();
    runtime
        .tick(&mut game, &mut motion, &mut world, &[], &mut switches)
        .unwrap();
    assert_eq!(switches[7], 1);
    released.zone_masks[1] = [0x100, 0, 0];
    released.zone_masks.push([0; 3]);
    assert!(released.publish_zone_interactions(&mut world).is_err());
    assert_eq!(released.zone_masks[1][0], 0x100);
    assert_eq!(world.entries[1].cell.interactions.enter, 0);
}

#[test]
fn released_zone_actions_match_original_machine_code() {
    let mut r = Reader(include_bytes!("fixtures/beta-zone-logic.bin"));
    assert_eq!(r.word(), 1664);
    for case in 0..1664 {
        let logic = Logic {
            code: r.word() as u8,
            condition: r.word() as u8,
            output: r.word() as u8,
            value: r.int(),
            events: [None, None],
        };
        let timer = r.int();
        let masks = r.interactions();
        let mut game = State::default();
        game.weapon.use_key = r.word() != 0;
        game.weapon.next = 3;
        let mut switches = std::array::from_fn(|_| r.word() as u8);
        for i in 0..16 {
            let amount = r.int();
            if i < 8 {
                game.inventory.amounts[i] = amount;
                game.inventory.maxima[i] = 100 + i as i32 * 10;
            } else {
                game.inventory.weapons[i - 8] = amount;
            }
        }
        let cells = vec![
            Cell::new(identity(), [20.; 3], 0),
            Cell::new(identity(), [2.; 3], 2),
        ];
        let mut world = World {
            connectivity: connectivity::connect(&cells),
            entries: cells
                .into_iter()
                .map(|cell| Entry {
                    cell,
                    sector: None,
                    events: [None, None],
                    logic: logic.clone(),
                })
                .collect(),
            adds: vec![0],
            subs: vec![],
            zones: vec![1],
            faces: vec![None, None],
        };
        world.entries[1].cell.interactions = masks;
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        motion.speed_up = 0.25;
        let mut runtime = Runtime::new_for_layout(&world, kkrieger_oxide::kx::Layout::Beta2004);
        runtime.respawn[0] = timer;
        assert!(
            runtime
                .tick(&mut game, &mut motion, &mut world, &[], &mut switches)
                .unwrap()
                .is_empty()
        );
        for (index, actual) in switches.iter().enumerate() {
            assert_eq!(*actual, r.word() as u8, "switch case {case} index {index}");
        }
        assert_eq!(game.weapon.next, r.word() as usize, "weapon case {case}");
        for i in 0..16 {
            let actual = if i < 8 {
                game.inventory.amounts[i]
            } else {
                game.inventory.weapons[i - 8]
            };
            assert_eq!(actual, r.int(), "inventory case {case} slot {i}");
        }
        assert_eq!(motion.speed_up.to_bits(), r.word(), "speed case {case}");
        assert_eq!(world.entries[1].cell.interactions, r.interactions());
        assert_eq!(runtime.respawn[0], r.int(), "timer case {case}");
    }
    assert!(r.0.is_empty());
}
