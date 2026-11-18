use kkrieger_oxide::game::beta_monster::dynamic::Pose;
#[test]
fn transformed_planes_match_released_dynamic_apply_matrix() {
    let bytes = include_bytes!("fixtures/beta-dynamic-planes.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let count = words.next().unwrap() as usize;
        let original: [[f32; 4]; 6] =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let pose = Pose::apply(&original[..count], matrix).unwrap();
        for (field, v) in pose.planes[..count]
            .iter()
            .flatten()
            .copied()
            .chain(pose.center)
            .chain(pose.matrix.into_iter().flatten())
            .enumerate()
        {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} field {field}"
            );
        }
        assert!(!pose.blocked);
    }
    assert!(words.next().is_none());
}
#[test]
fn dynamic_plane_capacity_is_checked() {
    let matrix = kkrieger_oxide::mesh::identity();
    assert!(Pose::apply(&[[1., 0., 0., 0.]; 7], matrix).is_err());
    let pose = Pose::apply(&[], matrix).unwrap();
    assert_eq!(pose.count, 0);
    assert_eq!(pose.center, [0., 0., 0., 1.]);
}
