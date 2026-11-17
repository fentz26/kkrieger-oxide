use kkrieger_oxide::{
    kx::{Key, Spline},
    random::Random,
    scene::scatter::{Parameters, State},
};
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
    fn check(&mut self, value: f32) {
        assert_eq!(value.to_bits(), self.word(), "{value}");
    }
}
#[test]
fn original_scene_particle_cache_positions_matrices_and_random_state_match_native() {
    let spline = Spline {
        interpolation: 1,
        channels: [(1., 3.), (2., 1.), (-1., 3.)]
            .into_iter()
            .map(|(a, b)| vec![Key { time: 0., value: a }, Key { time: 1., value: b }])
            .collect(),
    };
    let mut r = Reader(include_bytes!("fixtures/scene-particles.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 8);
    for case in 0..64 {
        let mut state = State::default();
        let mut random = Random::new(173);
        for step in 0..8 {
            let mode = r.word() as i32;
            let count = r.word() as i32;
            let seed = r.word() as i32;
            let has_spline = r.word() != 0;
            let spread = std::array::from_fn(|_| r.float());
            let animation = r.float();
            let line = std::array::from_fn(|_| r.float());
            let matrices = state
                .matrices(
                    Parameters {
                        mode,
                        count,
                        seed,
                        spread,
                    },
                    animation,
                    line,
                    has_spline.then_some(&spline),
                    &mut random,
                )
                .unwrap();
            assert_eq!(
                state.positions.len(),
                r.word() as usize,
                "case {case} step {step}"
            );
            for position in &state.positions {
                for value in position {
                    r.check(*value);
                }
            }
            assert_eq!(matrices.len(), r.word() as usize);
            for matrix in matrices {
                for vector in matrix {
                    for value in vector {
                        r.check(value);
                    }
                }
            }
            assert_eq!(
                random.next_u32(),
                r.word(),
                "random state {case} step {step}"
            );
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn scene_particle_handler_queues_each_child_instance_with_cached_phases() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::{Execution, Job},
    };
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.classes = [0x81, 0xc5]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    d.operators.truncate(2);
    for (index, op) in d.operators.iter_mut().enumerate() {
        op.class = index;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.splines.clear();
        op.parameters.clear();
    }
    d.operators[1].inputs = vec![0];
    d.operators[1].parameters = vec![
        0x20,
        3,
        17,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0.25f32.to_bits(),
        2f32.to_bits(),
        4f32.to_bits(),
        8f32.to_bits(),
    ];
    let mut e = Execution::new(&d);
    for _ in 0..2 {
        e.frame(&d, 1).unwrap();
        assert_eq!(e.jobs.len(), 3);
        for (index, job) in e.jobs.iter().enumerate() {
            let Job::Paint {
                operator, matrix, ..
            } = job
            else {
                panic!("expected particle child paint")
            };
            assert_eq!(*operator, 0);
            let mut time = 0.25 + index as f32 / 3.;
            if time >= 1. {
                time -= 1.;
            }
            assert_eq!(matrix[3], [2. * time, 4. * time, 8. * time, 1.]);
        }
    }
}
