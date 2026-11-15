use kkrieger_oxide::{
    collision::{
        Cell, DynamicCell, connectivity,
        world::{Entry, Logic, World},
    },
    game::monster_walk::{Context, DEFINITIONS, State},
    kx::{Key, Spline},
    mesh::{Matrix, identity},
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
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn matrix(&mut self) -> Matrix {
        std::array::from_fn(|_| self.vector())
    }
    fn cell(&mut self) -> Cell {
        let mode = self.word();
        Cell::from_vertices(std::array::from_fn(|_| self.vector()), mode)
    }
    fn check_float(&mut self, value: f32, label: &str) {
        let expected = self.float();
        assert!(
            value.to_bits() == expected.to_bits() || (value.is_nan() && expected.is_nan()),
            "{label}: {value:?} ({:08x}) vs {expected:?} ({:08x})",
            value.to_bits(),
            expected.to_bits()
        );
    }
    fn check_vector(&mut self, value: [f32; 4], label: &str) {
        for x in value {
            self.check_float(x, label);
        }
    }
    fn check_state(&mut self, s: &State, label: &str) {
        assert_eq!(s.reset, self.word() != 0, "{label}");
        for value in s.foot_cells {
            assert_eq!(value, self.reference(), "{label}: foot cell");
        }
        for v in s.old {
            self.check_vector(v, &format!("{label} old"));
        }
        for v in s.new {
            self.check_vector(v, &format!("{label} new"));
        }
        for value in s.step_time {
            assert_eq!(value, self.int(), "{label}: step time");
        }
        assert_eq!(s.last_time, self.int());
        for v in [s.old_position, s.new_position, s.center] {
            self.check_vector(v, &format!("{label} position"));
        }
        assert_eq!(s.state, self.word() as usize, "{label}: gait");
        assert_eq!(s.last_leg, self.word() as usize);
        assert_eq!(s.next_leg, self.word() as usize);
        assert_eq!(s.idle, self.word() != 0);
        self.check_vector(s.deviation, label);
        assert_eq!(s.deviation_flag, self.int());
        assert_eq!(s.init_steps, self.int());
        for v in s.delta {
            self.check_vector(v, &format!("{label} delta"));
        }
        for v in s.legs {
            self.check_vector(v, &format!("{label} legs"));
        }
        self.check_float(s.leg_time, label);
    }
    fn reference(&mut self) -> Option<usize> {
        let v = self.word();
        (v != u32::MAX).then_some(v as usize)
    }
}
#[test]
fn original_monster_walk_body_feet_gait_and_collision_match_native() {
    let mut r = Reader(include_bytes!("fixtures/monster-walk.bin"));
    assert_eq!(r.word(), 80);
    assert_eq!(r.word(), 32);
    let mut running = 0;
    for case in 0..80 {
        let count = r.word();
        let dynamic_count = r.word();
        let cells: Vec<_> = (0..count).map(|_| r.cell()).collect();
        let dynamic: Vec<_> = (0..dynamic_count)
            .map(|_| Rc::new(RefCell::new(DynamicCell::new(r.cell(), identity()))))
            .collect();
        let mut world = World {
            connectivity: connectivity::connect(&cells),
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
            faces: vec![Some(vec![]), Some(vec![]), None, None],
        };
        let style = r.word() as usize;
        let mut definition = DEFINITIONS[style];
        definition.flags = r.word();
        let use_spline = r.word() != 0;
        let curve = Spline {
            interpolation: case % 3,
            channels: [
                [0., 0.125, 0.],
                [0., 0.25, 0.],
                [0., -0.0625, 0.],
                [0., 1., 2.],
            ]
            .into_iter()
            .map(|values| {
                [0., 0.5, 1.]
                    .into_iter()
                    .zip(values)
                    .map(|(time, value)| Key { time, value })
                    .collect()
            })
            .collect(),
        };
        let mut state = State::default();
        for step in 0..32 {
            let time = r.int();
            let time_reset = r.word() != 0;
            let cell = r.reference();
            let mut matrix = r.matrix();
            state.deviation = r.vector();
            state.deviation_flag = r.int();
            state
                .advance(
                    &definition,
                    &mut matrix,
                    Context {
                        world: &mut world,
                        dynamic: &dynamic,
                        spline: use_spline.then_some(&curve),
                        cell,
                        time,
                        time_reset,
                    },
                )
                .unwrap_or_else(|e| panic!("{case}/{step}: {e}; cell {cell:?}"));
            let label = format!("{case}/{step}");
            for v in matrix {
                r.check_vector(v, &format!("{label} matrix"));
            }
            r.check_state(&state, &label);
            assert_eq!(r.word(), 0);
            running += usize::from(state.state == 1);
        }
    }
    assert!(running > 0);
    assert!(r.0.is_empty());
}
