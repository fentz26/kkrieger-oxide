use kkrieger_oxide::{
    kx::{Key, Spline},
    mesh::Matrix,
    scene::walk::{Parameters, State},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
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
    fn check(&mut self, v: [f32; 4]) {
        for value in v {
            let native = self.float();
            assert!(
                value.to_bits() == native.to_bits() || value.is_nan() && native.is_nan(),
                "{value} != {native}"
            );
        }
    }
}
#[test]
fn original_persistent_walker_body_legs_and_gait_state_match_native() {
    let spline = Spline {
        interpolation: 1,
        channels: [(0., 0.), (0., 0.5), (0., 0.), (0., 1.)]
            .into_iter()
            .map(|(a, b)| vec![Key { time: 0., value: a }, Key { time: 1., value: b }])
            .collect(),
    };
    let mut r = Reader(include_bytes!("fixtures/scene-walk.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 32);
    for case in 0..32 {
        let count = r.word() as usize;
        let flags = r.word();
        let has_spline = r.word() != 0;
        let run_group = r.word() as usize;
        let durations: [f32; 6] = std::array::from_fn(|_| r.float());
        let feet: [[f32; 3]; 8] = std::array::from_fn(|_| std::array::from_fn(|_| r.float()));
        let mut words = vec![0; 39];
        words[0] = flags;
        words[1] = count as u32;
        words[2] = 0.1f32.to_bits();
        words[3] = 0.3f32.to_bits();
        words[4] = 0.75f32.to_bits();
        words[5] = 2;
        words[6] = run_group as u32;
        for (i, value) in durations.into_iter().enumerate() {
            words[7 + i] = value.to_bits();
        }
        for (i, foot) in feet.into_iter().enumerate() {
            for (axis, value) in foot.into_iter().enumerate() {
                words[13 + i * 3 + axis] = value.to_bits();
            }
        }
        let p = Parameters::from_words(&words).unwrap();
        let mut state = State::default();
        for step in 0..32 {
            let now = r.word() as i32;
            let reset = r.word() != 0;
            let parent = r.matrix();
            let frame = state
                .advance(&p, parent, now, reset, has_spline.then_some(&spline))
                .unwrap();
            for vector in frame.transform {
                r.check(vector);
            }
            r.check(frame.time);
            for &foot in &frame.feet[..frame.count] {
                r.check(foot);
            }
            for value in [
                state.state as i32,
                state.last_leg as i32,
                state.next_leg as i32,
                i32::from(state.idle),
                state.last_time,
                state.initial_steps,
            ] {
                assert_eq!(value, r.word() as i32, "case {case} step {step}");
            }
            r.check(state.new_position);
            r.check(state.center);
            for i in 0..count {
                r.check(state.old[i]);
                r.check(state.new[i]);
                r.check(state.delta[i]);
                assert_eq!(state.step_time[i], r.word() as i32);
            }
            r.check([27., 28., 29., 30.]);
            r.check([16., 17., 18., 19.]);
            assert_eq!(r.word(), 0);
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn walker_child_snapshots_and_variable_restoration_follow_native_scope() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::{Execution, Job},
    };
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.classes = [0xc8, 0xc1, 0x64, 0xff]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    d.operators.truncate(3);
    for (index, op) in d.operators.iter_mut().enumerate() {
        op.class = index;
        op.inputs.clear();
        op.links.clear();
        op.splines.clear();
        op.animation = &[1];
        op.parameters.clear();
    }
    d.operators[0].inputs = vec![1];
    d.operators[1].inputs = vec![2];
    let mut parameters = vec![0; 39];
    parameters[1] = 2;
    parameters[2] = 0.1f32.to_bits();
    parameters[3] = 0.3f32.to_bits();
    parameters[4] = 0.75f32.to_bits();
    parameters[5] = 2;
    parameters[6] = 2;
    for (index, value) in [0.25f32, 0.5, 0.25, 0.125, 0.125, 0.0625]
        .into_iter()
        .enumerate()
    {
        parameters[7 + index] = value.to_bits();
    }
    parameters[13] = (-0.5f32).to_bits();
    parameters[16] = 0.5f32.to_bits();
    d.operators[0].parameters = parameters;
    d.operators[2].animation = &[2, 16, 0x91, 0, 2, 27, 0x91, 1, 1];
    let mut e = Execution::new(&d);
    e.vm.variables.fill([9.; 4]);
    e.frame(&d, 0).unwrap();
    let Job::Paint {
        matrix,
        effect_variables: Some(snapshot),
        ..
    } = &e.jobs[0]
    else {
        panic!("missing walker child")
    };
    assert_eq!(*matrix, kkrieger_oxide::mesh::identity());
    assert_eq!(snapshot.start, 16);
    assert_eq!(
        snapshot.values[0],
        [-0.5, (std::f64::consts::PI.sin() * 0.25) as f32, 0., 1.]
    );
    assert_eq!(
        snapshot.values[1],
        [0.5, (std::f64::consts::PI.sin() * 0.25) as f32, 0., 1.]
    );
    assert_eq!(snapshot.values[11], [0., 0., 0., 0.]);
    assert_eq!(&e.vm.variables[16..18], &[[9.; 4]; 2]);
    assert_eq!(e.vm.variables[27], [9.; 4]);
    d.operators[2].class = 3;
    d.operators[2].animation = &[1];
    let mut e = Execution::new(&d);
    e.vm.variables.fill([9.; 4]);
    assert!(e.frame(&d, 0).is_err());
    assert_eq!(&e.vm.variables[16..18], &[[9.; 4]; 2]);
    assert_eq!(e.vm.variables[27], [9.; 4]);
}
