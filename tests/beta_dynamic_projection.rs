use kkrieger_oxide::game::beta_monster::dynamic::project_particle;
#[test]
fn moving_cell_projection_matches_actual_math_and_entry_query() {
    let bytes = include_bytes!("fixtures/beta-dynamic-projection.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    let planes = [
        [1., 0., 0., 2.],
        [-1., 0., 0., 2.],
        [0., 1., 0., 2.],
        [0., -1., 0., 2.],
        [0., 0., 1., 2.],
        [0., 0., -1., 2.],
    ];
    for case in 0..128 {
        let point = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let delta =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let blocked = words.next().unwrap() != 0;
        let expected: [u32; 4] = std::array::from_fn(|_| words.next().unwrap());
        let result = project_particle(point, delta, &planes);
        assert_eq!(result.is_none(), blocked, "case {case} blocked");
        assert_eq!(
            result.map(|p| p.map(f32::to_bits)).unwrap_or([0; 4]),
            expected,
            "case {case} point"
        );
    }
    assert!(words.next().is_none());
}
