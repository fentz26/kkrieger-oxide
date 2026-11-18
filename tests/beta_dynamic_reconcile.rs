use kkrieger_oxide::{
    game::beta_monster::{
        body::Body,
        collision::Completion,
        dynamic::{MovingCell, Pose},
    },
    mesh::Matrix,
};
#[test]
fn dynamic_reconciliation_matches_native_filters_probes_and_commit() {
    let bytes = include_bytes!("fixtures/beta-dynamic-reconcile.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    let original = [
        [1., 0., 0., 1.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    for case in 0..128 {
        let kind = words.next().unwrap();
        let owned = words.next().unwrap() != 0;
        let monster = words.next().unwrap() != 0;
        let status = words.next().unwrap();
        let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let confirmed: Matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let current: Matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let changes: [[f32; 4]; 3] =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let mut body = Body::new(position, Some(0), kind as i32);
        let p = &mut body.particles[0];
        p.vectors = [position; 4];
        let mut cell = MovingCell {
            original,
            count: 6,
            confirmed,
            current,
            mode: 0x21,
            monster,
            pose: Pose::apply(&original, confirmed).unwrap(),
        };
        let mut calls = 0;
        let mut probe = [0u32; 4];
        cell.reconcile(std::iter::once((&mut *p, owned)), |p| {
            calls += 1;
            probe = p.vectors[0].map(f32::to_bits);
            p.vectors[..3].copy_from_slice(&changes);
            Ok(match status {
                0 => Completion::Clear,
                1 => Completion::Contact,
                _ => Completion::Blocked,
            })
        })
        .unwrap();
        assert_eq!(calls, words.next().unwrap(), "case {case} calls");
        let expected: [u32; 4] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(probe, expected, "case {case} probe");
        for (field, v) in p.vectors[..3].iter().flatten().enumerate() {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} particle field {field}"
            );
        }
        assert_eq!(cell.mode, words.next().unwrap(), "case {case} mode");
        assert_eq!(
            u32::from(cell.pose.blocked),
            words.next().unwrap(),
            "case {case} blocked"
        );
        for (field, v) in cell
            .pose
            .planes
            .iter()
            .flatten()
            .copied()
            .chain(cell.pose.center)
            .chain(cell.confirmed.into_iter().flatten())
            .chain(cell.current.into_iter().flatten())
            .enumerate()
        {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} cell field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}
#[test]
fn probe_error_restores_position_and_blocks_cell() {
    let original = [
        [1., 0., 0., 1.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let confirmed = kkrieger_oxide::mesh::identity();
    let mut current = confirmed;
    current[3][0] = 0.2;
    let mut cell = MovingCell {
        original,
        count: 6,
        confirmed,
        current,
        mode: 0x21,
        monster: false,
        pose: Pose::apply(&original, confirmed).unwrap(),
    };
    let mut body = Body::new([0., 0., 0., 1.], Some(0), 8);
    let p = &mut body.particles[0];
    p.vectors[0][3] = 1.; // PhysicPart supplies a homogeneous position before this phase.
    let before = p.vectors[0];
    assert_eq!(
        cell.reconcile(std::iter::once((&mut *p, false)), |p| {
            p.vectors[0] = [9.; 4];
            p.vectors[2][0] = 7.;
            Err("probe failed")
        }),
        Err("probe failed")
    );
    assert_eq!(p.vectors[0], before);
    assert_eq!(p.vectors[2][0], 7.);
    assert!(cell.pose.blocked);
    assert_eq!(cell.confirmed, confirmed);
    assert_eq!(cell.mode, 0x21);
}
