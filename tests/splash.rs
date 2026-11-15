use kkrieger_oxide::{
    collision::{
        Cell, DynamicCell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        State,
        monster::Health,
        splash::{Splash, Targets},
    },
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
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn cell(&mut self) -> Cell {
        let mode = self.word();
        Cell::from_vertices(std::array::from_fn(|_| self.vector()), mode)
    }
}
#[test]
fn original_splash_phase_preserves_visibility_falloff_damage_order_and_switches() {
    let mut r = Reader(include_bytes!("fixtures/splash.bin"));
    assert_eq!(r.word(), 64);
    let mut hurt = 0;
    let mut alerted_without_damage = 0;
    for case in 0..64 {
        let n = r.word();
        let cells: Vec<_> = (0..n).map(|_| r.cell()).collect();
        let n = r.word();
        let dynamic: Vec<_> = (0..n)
            .map(|_| {
                let cell = r.cell();
                let mode = cell.mode;
                let mut cell = DynamicCell::new(cell, identity());
                cell.cell.mode = mode;
                Rc::new(RefCell::new(cell))
            })
            .collect();
        let graph = connectivity::connect(&cells);
        let mut world = World {
            entries: cells
                .into_iter()
                .map(|cell| Entry {
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
                })
                .collect(),
            adds: vec![0, 1],
            subs: vec![2],
            zones: vec![3],
            connectivity: graph,
            faces: vec![Some(vec![]), Some(vec![]), None, None],
        };
        let mut splash = Splash {
            enabled: r.word() != 0,
            hits: r.word() as i32,
            range: r.float(),
            position: r.vector(),
        };
        let player = r.vector();
        let mut game = State::default();
        game.inventory.amounts[0] = r.word() as i32;
        game.inventory.amounts[1] = r.word() as i32;
        let n = r.word();
        let mut positions = Vec::new();
        for _ in 0..n {
            game.monsters.push(Health {
                life: r.word() as i32,
                armor: r.word() as i32,
                state: r.word() as i32,
                kill_switch: r.word() as i32,
            });
            positions.push(r.vector());
        }
        let old: Vec<_> = game.monsters.iter().map(|m| m.life).collect();
        let mut switches = [0; 256];
        switches[7] = 254;
        switches[255] = 255;
        let sound = game
            .apply_splash(
                &mut splash,
                Targets {
                    monsters: &positions,
                    player,
                },
                &mut world,
                &dynamic,
                &mut switches,
            )
            .unwrap();
        assert_eq!(splash.enabled, r.word() != 0);
        assert_eq!(splash.hits, r.word() as i32);
        assert_eq!(game.inventory.amounts[0], r.word() as i32, "case {case}");
        assert_eq!(game.inventory.amounts[1], r.word() as i32);
        assert_eq!(u32::from(sound.is_some()), r.word());
        if let Some(s) = sound {
            assert_eq!(s.slot, 10);
            assert_eq!(s.volume, 1.);
            assert_eq!(s.far_distance, 160.);
            hurt += 1;
        }
        for (m, old) in game.monsters.iter().zip(old) {
            assert_eq!(m.life, r.word() as i32, "case {case}");
            assert_eq!(m.armor, r.word() as i32);
            assert_eq!(m.state, r.word() as i32);
            if m.state == 4 && m.life == old {
                alerted_without_damage += 1;
            }
        }
        assert_eq!(switches[7], r.word() as u8);
        assert_eq!(switches[255], r.word() as u8);
    }
    assert!(hurt > 0 && alerted_without_damage > 0);
    assert!(r.0.is_empty());
}

#[test]
fn original_splash_trigger_preserves_rising_edge_and_pending_payload() {
    let mut r = Reader(include_bytes!("fixtures/splash-trigger.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 8);
    for _ in 0..64 {
        let mut splash = Splash::default();
        let mut previous = false;
        for _ in 0..8 {
            if r.word() != 0 {
                splash.enabled = false;
            }
            let hits = r.word() as i32;
            let range = r.float();
            let enable = r.float();
            let position = r.vector();
            splash.trigger(&mut previous, hits, range, enable, position);
            assert_eq!(previous, r.word() != 0);
            assert_eq!(splash.enabled, r.word() != 0);
            assert_eq!(splash.hits, r.word() as i32);
            assert_eq!(splash.range.to_bits(), r.word());
            for (a, b) in splash.position.into_iter().zip(r.vector()) {
                assert_eq!(a.to_bits(), b.to_bits());
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn scene_splash_handler_uses_transformed_position_and_instance_memory() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.classes = vec![
        Class {
            id: 0xc3,
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
    doc.operators[0].parameters = [1f32, 1., 1., 0., 0., 0., 2., 3., 4.]
        .map(f32::to_bits)
        .to_vec();
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![50, 8f32.to_bits(), 1f32.to_bits()];
    let mut execution = Execution::new(&doc);
    execution.frame(&doc, 0).unwrap();
    assert!(execution.splash.enabled);
    assert_eq!(execution.splash.position, [2., 3., 4., 1.]);
    assert_eq!(execution.splash.hits, 50);
    execution.splash.enabled = false;
    execution.frame(&doc, 0).unwrap();
    assert!(!execution.splash.enabled);
}
