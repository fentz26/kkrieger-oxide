use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{monster_actor::Actor, monster_walk::State},
    kx::{Class, Document},
    mesh::identity,
    scene::{
        event::Event,
        exec::{ActorContext, Execution},
    },
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
fn original_complete_monster_scene_handler_matches_native() {
    let mut r = Reader(include_bytes!("fixtures/monster-scene.bin"));
    assert_eq!(r.word(), 20);
    assert_eq!(r.word(), 6);
    for case in 0..20 {
        let words: Vec<_> = (0..29).map(|_| r.word()).collect();
        let srt: Vec<_> = (0..9).map(|_| r.word()).collect();
        let player_position = r.vector();
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        doc.events.clear();
        doc.classes = [0x11, 0x14, 0xc3, 0x15]
            .into_iter()
            .map(|id| Class {
                id,
                convention: 0,
                packing: &[],
            })
            .collect();
        doc.operators.truncate(5);
        for op in &mut doc.operators {
            op.animation = &[1];
            op.inputs.clear();
            op.links.clear();
            op.splines.clear();
        }
        doc.operators[0].class = 0;
        doc.operators[0].parameters = words;
        doc.operators[0].inputs = vec![1, 2];
        for (i, hits) in [(1, 10), (2, 20)] {
            doc.operators[i].class = 1;
            doc.operators[i].parameters = vec![hits, 8f32.to_bits(), 1f32.to_bits()];
        }
        doc.operators[3].class = 2;
        doc.operators[3].parameters = srt;
        doc.operators[3].inputs = vec![0];
        doc.operators[4].class = 3;
        doc.operators[4].parameters = vec![7, 9, 4];
        doc.operators[4].inputs = vec![3];
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
            player_position,
            player_hit: None,
            time_reset: false,
        });
        for step in 0..6 {
            if step > 0 {
                let actor = execution.game.monsters.get(0).unwrap();
                let mut actor = actor.borrow_mut();
                actor.health.state = match step {
                    1 | 5 => 3,
                    2 | 3 => 4,
                    _ => 0,
                };
                if step == 3 {
                    actor.health.life = 0;
                }
                if [1, 2, 5].contains(&step) {
                    actor.event.borrow_mut().matrix = actor.matrix;
                }
            }
            execution.current_time = step * 10;
            execution.vm.variables[27] = [7., 8., 9., 10.];
            execution.splash.enabled = false;
            execution.frame(&doc, 4).unwrap();
            let actor = execution.game.monsters.get(0).unwrap();
            let actor = actor.borrow();
            let label = format!("{case}/{step}");
            r.check_actor(&actor, &label);
            assert_eq!(execution.game.monsters.len(), r.word() as usize);
            assert_eq!(execution.dynamic_cells.len(), r.word() as usize);
            let expected_call = actor.health.life <= 0 || actor.health.state >= 3;
            assert_eq!(u32::from(expected_call), r.word());
            assert_eq!(
                r.word(),
                if !expected_call {
                    0
                } else if actor.health.life <= 0 {
                    102
                } else {
                    101
                }
            );
            assert_eq!(execution.splash.enabled, r.word() != 0);
            assert_eq!(execution.splash.hits, r.int());
            r.check_float(execution.splash.range, &label);
            r.check_vector(execution.splash.position, &label);
            assert_eq!(execution.next_monster_override.is_some(), r.word() != 0);
            for &value in &execution.vm.variables[16..24] {
                r.check_vector(value, &label);
            }
            r.check_vector(execution.vm.variables[27], &label);
            assert_eq!(execution.has_event_instances(0), r.word() != 0);
            assert_eq!(execution.has_event_instances(1), r.word() != 0);
        }
    }
    assert!(r.0.is_empty());
}
