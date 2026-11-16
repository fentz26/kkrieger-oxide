use kkrieger_oxide::{
    kx::{Class, Document},
    mesh::identity,
    scene::exec::{Execution, Job},
};
#[test]
fn rotate_aligns_each_axis_and_uses_current_camera_for_other_modes() {
    for axis in 0..5 {
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        doc.events.clear();
        doc.classes = [0xc3, 0xc9, 0xc0, 0x61]
            .into_iter()
            .map(|id| Class {
                id,
                convention: 0,
                packing: &[],
            })
            .collect();
        doc.operators.truncate(4);
        for (index, op) in doc.operators.iter_mut().enumerate() {
            op.animation = &[1];
            op.inputs.clear();
            op.links.clear();
            op.class = index;
        }
        doc.operators[0].parameters = [1f32, 1., 1., 0., 0., 0., 3., 4., 5.]
            .map(f32::to_bits)
            .to_vec();
        doc.operators[0].inputs = vec![1];
        let direction = match axis {
            0 => [1f32, 0., 0.],
            1 => [0., 1., 0.],
            _ => [0., 0., 1.],
        };
        doc.operators[1].parameters = direction.map(f32::to_bits).to_vec();
        doc.operators[1].parameters.push(axis);
        doc.operators[1].inputs = vec![2];
        doc.operators[2].parameters = [1f32, 1., 1., 0., 0., 0., 0., 0., 0.]
            .map(f32::to_bits)
            .to_vec();
        doc.operators[2].inputs = vec![3];
        let mut e = Execution::new(&doc);
        e.current_camera.space = [
            [0., 1., 0., 0.],
            [-1., 0., 0., 0.],
            [0., 0., 1., 0.],
            [9., 8., 7., 1.],
        ];
        e.frame(&doc, 0).unwrap();
        let Job::Paint { matrix, .. } = e.jobs[0] else {
            panic!("missing paint")
        };
        let mut expected = if axis < 3 {
            identity()
        } else {
            e.current_camera.space
        };
        expected[3] = [3., 4., 5., 1.];
        for (a, b) in matrix
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            assert_eq!(a, b);
        }
    }
}

#[test]
fn original_rotation_transforms_match_native_at_input_boundary() {
    struct Reader<'a>(&'a [u8]);
    impl Reader<'_> {
        fn word(&mut self) -> u32 {
            let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            value
        }
        fn float(&mut self) -> f32 {
            f32::from_bits(self.word())
        }
        fn matrix(&mut self) -> [[f32; 4]; 4] {
            std::array::from_fn(|_| std::array::from_fn(|_| self.float()))
        }
    }
    let mut r = Reader(include_bytes!("fixtures/scene-rotate.bin"));
    assert_eq!(r.word(), 512);
    for _ in 0..512 {
        let axis = r.word() as i32;
        let direction = std::array::from_fn(|_| r.float());
        let parent = r.matrix();
        let camera = r.matrix();
        let actual = kkrieger_oxide::scene::camera::rotate(parent, camera, direction, axis);
        let expected = r.matrix();
        for (a, b) in actual
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            if b.is_nan() {
                assert!(a.is_nan());
            } else {
                assert_eq!(a.to_bits(), b.to_bits());
            }
        }
        assert_eq!(r.word(), 1);
        assert_eq!(parent, r.matrix());
    }
    assert!(r.0.is_empty());
}
