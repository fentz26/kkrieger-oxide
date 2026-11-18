use kkrieger_oxide::game::beta_monster::dynamic::interpolate;
#[test]
fn dynamic_interpolation_matches_released_basis_reconstruction() {
    let bytes = include_bytes!("fixtures/beta-dynamic-fade.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let first =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let second =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let factor = f32::from_bits(words.next().unwrap());
        for (field, v) in interpolate(first, second, factor)
            .into_iter()
            .flatten()
            .enumerate()
        {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}
#[test]
fn interpolated_frame_commits_confirmed_pose_without_particles() {
    use kkrieger_oxide::game::beta_monster::dynamic::{MovingCell, Pose};
    let original = [
        [1., 0., 0., 1.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let first = kkrieger_oxide::mesh::identity();
    let mut second = first;
    second[3][0] = 0.4;
    let mut cell = MovingCell {
        original,
        count: 6,
        confirmed: first,
        current: first,
        mode: 0x21,
        monster: false,
        pose: Pose::apply(&original, first).unwrap(),
    };
    cell.reconcile_frame(first, second, 0.5, std::iter::empty(), |_| {
        panic!("unexpected particle probe")
    })
    .unwrap();
    assert_eq!(cell.mode, 1);
    assert_eq!(cell.confirmed, cell.current);
    assert_eq!(cell.pose.center[0].to_bits(), 0.2f32.to_bits());
    assert!(!cell.pose.blocked);
}
