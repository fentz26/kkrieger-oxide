use kkrieger_oxide::{
    collision::{
        Cell, DynamicCell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        Events, Parameters,
        shot::{self, Damage, Origin},
    },
    mesh::identity,
    random::Random,
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
    fn optional(&mut self) -> Option<usize> {
        let v = self.word();
        (v != u32::MAX).then_some(v as usize)
    }
    fn cell(&mut self) -> Cell {
        let mode = self.word();
        Cell::from_vertices(std::array::from_fn(|_| self.vector()), mode)
    }
    fn check(&mut self, v: [f32; 4], case: usize, step: usize) {
        for (a, b) in v.into_iter().zip(self.vector()) {
            assert_eq!(a.to_bits(), b.to_bits(), "{case}/{step}: {a} vs {b}");
        }
    }
}
#[test]
fn original_projectile_trajectories_preserve_bounces_damage_events_and_zones() {
    let mut r = Reader(include_bytes!("fixtures/shot-ai.bin"));
    assert_eq!(r.word(), 64);
    let mut monster_hits = 0;
    let mut player_hits = 0;
    let mut explosions = 0;
    let mut steps = 0;
    for case in 0..64 {
        let count = r.word();
        let cells: Vec<_> = (0..count).map(|_| r.cell()).collect();
        let count = r.word();
        let dynamic: Vec<_> = (0..count)
            .map(|_| {
                let cell = r.cell();
                let mode = cell.mode;
                let mut cell = DynamicCell::new(cell, identity());
                cell.cell.mode = mode;
                let monster = r.word();
                cell.monster = (monster != 0).then_some(monster as usize);
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
        let weapon = r.word() as usize;
        let monster = r.word() != 0;
        let seed = r.word() as i32;
        let direction = r.float();
        let mut parameters = Parameters::default();
        parameters.weapon_hits[weapon] = r.word() as i32;
        let mut events = Events::default();
        events.explode[0][weapon] = r.optional();
        events.explode[1][weapon] = r.optional();
        let origin = if monster {
            Origin::Monster {
                id: 42,
                position: [0., 0., 0., 1.],
                direction: [0., 0., 1., 0.],
                cell: 0,
            }
        } else {
            Origin::Player {
                position: [0., 0., 0., 1.],
                look: 0.,
                direction,
                last_camera_y: 0.,
                camera_offset: 0.,
                cell: 0,
            }
        };
        let mut rng = Random::new(seed);
        let shot = shot::spawn(weapon, Some(71), origin, &mut rng).unwrap();
        assert_eq!(shot.is_some(), r.word() != 0);
        if let Some(mut shot) = shot {
            shot.double_kill = r.word() as i32;
            let mut step = 0;
            while r.word() != 0 {
                let out = shot
                    .advance(
                        &mut world,
                        &dynamic,
                        Some(&dynamic[0]),
                        &parameters,
                        &events,
                    )
                    .unwrap();
                assert_eq!(out.alive, r.word() != 0);
                assert_eq!(shot.cell, r.word() as usize);
                assert_eq!(shot.double_kill, r.word() as i32);
                r.check(shot.old_position, case, step);
                r.check(shot.tensor, case, step);
                for v in shot.matrix {
                    r.check(v, case, step);
                }
                let kind = r.word();
                match (out.damage, kind) {
                    (None, 0) => {}
                    (Some(Damage::Monster { monster, hits: n }), 1) => {
                        assert_eq!(n, r.word() as i32);
                        assert_eq!(monster, r.word() as usize);
                        monster_hits += 1;
                    }
                    (Some(Damage::Player { hits: n }), 2) => {
                        assert_eq!(n, r.word() as i32);
                        player_hits += 1;
                    }
                    _ => panic!("{case}/{step}: damage kind {kind}"),
                }
                assert_eq!(usize::from(out.explosion.is_some()), r.word() as usize);
                if let Some(event) = out.explosion {
                    assert_eq!(event.operator, r.optional());
                    for v in event.matrix {
                        r.check(v, case, step);
                    }
                    explosions += 1;
                }
                for &i in &world.connectivity.sorted {
                    let state = world.entries[i].cell.interactions;
                    assert_eq!(
                        [state.enter, state.leave, state.collided],
                        [r.word(), r.word(), r.word()]
                    );
                }
                for cell in &dynamic {
                    let state = cell.borrow().cell.interactions;
                    assert_eq!(
                        [state.enter, state.leave, state.collided],
                        [r.word(), r.word(), r.word()]
                    );
                }
                step += 1;
                steps += 1;
            }
        }
    }
    assert!(monster_hits > 0 && player_hits > 0 && explosions > 0 && steps > 64);
    assert!(r.0.is_empty());
}
