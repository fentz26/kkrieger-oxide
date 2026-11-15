use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        monster_actor::{Actor, Context, Override},
        monster_walk::State,
    },
    kx::{Key, Spline},
    mesh::Matrix,
    scene::event::Event,
};
use std::rc::Rc;
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

impl Reader<'_> {
    fn check_event(&mut self, event: &Event, id: usize, label: &str) {
        for v in event.matrix {
            self.check_vector(v, label);
        }
        assert_eq!(event.operator, self.reference());
        assert_eq!(event.monster == Some(id), self.word() != 0);
        for v in [event.scale, event.rotate, event.translate] {
            self.check_vector(v, label);
        }
        for value in [
            event.velocity,
            event.modulation,
            event.interval[0],
            event.interval[1],
        ] {
            self.check_float(value, label);
        }
        for value in [event.select, event.start, event.end, event.cull_distance] {
            assert_eq!(value, self.int(), "{label}");
        }
        assert_eq!(event.color, self.word());
        assert_eq!(event.spline.is_some(), self.word() != 0);
    }
    fn check_actor(&mut self, a: &Actor, label: &str) {
        for v in a.matrix {
            self.check_vector(v, label);
        }
        for value in [
            a.parameters.speed,
            a.parameters.notice_radius,
            a.parameters.charge_radius,
        ] {
            self.check_float(value, label);
        }
        for value in [
            a.parameters.flags,
            a.parameters.life_max,
            a.health.armor,
            a.parameters.melee_hits,
        ] {
            assert_eq!(value, self.int(), "{label}");
        }
        self.check_float(a.parameters.melee_time, label);
        assert_eq!(a.parameters.ranged_hits, self.int());
        self.check_float(a.parameters.ranged_time, label);
        self.check_float(a.parameters.ranged_aim, label);
        assert_eq!(a.health.kill_switch, self.int());
        self.check_float(a.parameters.wait_radius, label);
        self.check_float(a.parameters.min_radius, label);
        self.check_event(&a.event.borrow(), a.id, label);
        self.check_event(&a.death.borrow(), a.id, label);
        assert_eq!(a.health.life, self.int());
        self.check_float(a.melee_timer, label);
        self.check_float(a.ranged_timer, label);
        assert_eq!(a.health.state, self.int());
        assert_eq!(a.walk_style, self.word() as usize);
        assert_eq!(a.weapon_kind, self.int());
        assert_eq!(a.spline.is_some(), self.word() != 0);
        for v in [
            a.magnet_force,
            a.position_offset,
            a.body.position,
            a.body.old_position,
        ] {
            self.check_vector(v, label);
        }
        assert_eq!(a.body.cell, self.reference());
        self.check_float(a.body.radius, label);
        let hit = a.hit_cell.borrow();
        assert_eq!(hit.cell.mode, self.word());
        assert_eq!(hit.monster == Some(a.id), self.word() != 0);
        assert_eq!(hit.blocked, self.int());
        assert_eq!(hit.cell.planes.len(), self.word() as usize);
        for &v in &hit.cell.planes {
            self.check_vector(v, label);
        }
        for v in hit.cell.vertices {
            self.check_vector(v, label);
        }
        for matrix in [
            hit.matrix0,
            hit.matrix1,
            hit.confirmed_matrix,
            hit.current_matrix,
        ] {
            for v in matrix {
                self.check_vector(v, label);
            }
        }
        self.check_state(&a.walk, label);
        assert_eq!(a.spawn_switch, self.int());
    }
}
#[test]
fn original_monster_initialization_and_respawn_match_native() {
    let mut r = Reader(include_bytes!("fixtures/monster-actor.bin"));
    assert_eq!(r.word(), 40);
    assert_eq!(r.word(), 3);
    for case in 0..40 {
        let count = r.word();
        let cells: Vec<_> = (0..count).map(|_| r.cell()).collect();
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
            zones: vec![],
            faces: vec![Some(vec![]), Some(vec![]), None],
        };
        let words: [u32; 29] = std::array::from_fn(|_| r.word());
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
        let mut actor: Option<Actor> = None;
        let mut hit = None;
        for step in 0..3 {
            let time = r.int();
            let apply_override = r.word() != 0;
            let flags = Override {
                kill_switch: r.int(),
                spawn_switch: r.int(),
                flags: r.int(),
            };
            let player_position = r.vector();
            let parent = r.matrix();
            let context = Context {
                world: &mut world,
                dynamic: &[],
                player_position,
                parent,
                spline: use_spline.then_some((42, &curve)),
                time,
                time_reset: false,
                flags: apply_override.then_some(flags),
            };
            if let Some(a) = &mut actor {
                a.health.state = 0;
                a.melee_timer = step as f32 * 0.125;
                a.ranged_timer = step as f32 * 0.25;
                a.magnet_force = [step as f32 * 0.1, -0.25, 0.125, 0.];
                a.initialize(&words, context).unwrap();
                assert!(Rc::ptr_eq(hit.as_ref().unwrap(), &a.hit_cell));
            } else {
                let a = Actor::spawn(77, &words, context).unwrap();
                hit = Some(Rc::clone(&a.hit_cell));
                actor = Some(a);
            }
            let a = actor.as_ref().unwrap();
            r.check_actor(a, &format!("{case}/{step}"));
            assert_eq!(r.word(), 0);
            assert_eq!(a.event.borrow().matrix[0][0], f32::from_bits(words[20]));
            assert_eq!(a.event.borrow().matrix[0][1], f32::from_bits(words[21]));
        }
    }
    assert!(r.0.is_empty());
}
