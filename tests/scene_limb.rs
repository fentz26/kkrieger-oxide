use kkrieger_oxide::{
    mesh::{Matrix, identity},
    scene::limb,
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
    fn matrix(&mut self) -> Matrix {
        std::array::from_fn(|_| std::array::from_fn(|_| self.float()))
    }
    fn check_matrix(&mut self, matrix: Matrix) {
        for vector in matrix {
            for value in vector {
                let native = self.float();
                assert!(
                    value.to_bits() == native.to_bits() || value.is_nan() && native.is_nan(),
                    "{value} != {native}"
                );
            }
        }
    }
}
#[test]
fn original_limb_world_relative_reach_and_matrix_variables_match_native() {
    let mut r = Reader(include_bytes!("fixtures/scene-limb.bin"));
    assert_eq!(r.word(), 256);
    for case in 0..256 {
        let count = r.word();
        let parent = r.matrix();
        let position = std::array::from_fn(|_| r.float());
        let direction = std::array::from_fn(|_| r.float());
        let a = r.float();
        let b = r.float();
        let flags = r.word();
        let count_native = r.word();
        if count == 2 {
            assert_eq!(count_native, 2, "case {case}");
            for (index, joint) in limb::joints(parent, position, direction, a, b, flags)
                .into_iter()
                .enumerate()
            {
                assert_eq!(r.word(), index as u32);
                r.check_matrix(joint.transform);
                r.check_matrix(joint.other);
            }
        } else {
            assert_eq!(count_native, 0);
        }
        let mut restored = identity();
        restored[3] = [17., 19., 23., 1.];
        r.check_matrix(restored);
        assert_eq!(r.word(), 0);
    }
    assert!(r.0.is_empty());
}

#[test]
fn limb_children_capture_opposite_joint_and_restore_variables_on_failure() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::{Execution, Job},
    };
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.classes = [0xc7, 0x64, 0xff]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    d.operators.truncate(3);
    for op in &mut d.operators {
        op.inputs.clear();
        op.links.clear();
        op.splines.clear();
        op.animation = &[1];
        op.parameters.clear();
        op.class = 1;
    }
    d.operators[0].class = 0;
    d.operators[0].inputs = vec![1, 2];
    d.operators[0].parameters = vec![
        0,
        (-1.5f32).to_bits(),
        0,
        1f32.to_bits(),
        0,
        0,
        1f32.to_bits(),
        1f32.to_bits(),
        2,
    ];
    for op in &mut d.operators[1..] {
        op.animation = &[2, 28, 0x91, 0, 2, 31, 0x91, 1, 1];
    }
    let joints = limb::joints(identity(), [0., -1.5, 0.], [1., 0., 0.], 1., 1., 2);
    let mut e = Execution::new(&d);
    e.vm.variables[28..32].fill([9.; 4]);
    e.frame(&d, 0).unwrap();
    assert_eq!(e.jobs.len(), 2);
    for (job, joint) in e.jobs.iter().zip(joints) {
        let Job::Paint {
            matrix,
            effect_variables: Some(snapshot),
            ..
        } = job
        else {
            panic!("missing limb child")
        };
        assert_eq!(*matrix, joint.transform);
        assert_eq!(snapshot.start, 28);
        assert_eq!(snapshot.values, joint.other);
    }
    assert_eq!(&e.vm.variables[28..32], &[[9.; 4]; 4]);
    d.operators[2].class = 2;
    d.operators[2].animation = &[1];
    let mut e = Execution::new(&d);
    e.vm.variables[28..32].fill([9.; 4]);
    assert!(e.frame(&d, 0).is_err());
    assert!(e.jobs.is_empty());
    assert_eq!(&e.vm.variables[28..32], &[[9.; 4]; 4]);
    d.operators[0].inputs.pop();
    let mut e = Execution::new(&d);
    e.frame(&d, 0).unwrap();
    assert!(e.jobs.is_empty());
}
