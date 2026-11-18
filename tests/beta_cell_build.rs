use kkrieger_oxide::game::beta_monster::hull::Hull;
#[test]
fn collision_cell_geometry_matches_complete_released_constructor() {
    let bytes = include_bytes!("fixtures/beta-cell-build.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let mode = words.next().unwrap();
        let vertices =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let count = words.next().unwrap() as usize;
        let cell = Hull::from_vertices(vertices, mode);
        assert_eq!(cell.count, count, "case {case} planes");
        for (field, v) in cell.planes[..count]
            .iter()
            .flatten()
            .copied()
            .chain(cell.bounds.into_iter().flatten())
            .chain(cell.center)
            .enumerate()
        {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} field {field}"
            );
        }
        assert_eq!(cell.mode, words.next().unwrap(), "case {case} mode");
        assert_eq!(
            u32::from(cell.only_for_shot),
            words.next().unwrap(),
            "case {case} shot flag"
        );
    }
    assert!(words.next().is_none());
}
#[test]
fn released_hitbox_builds_subtractive_collision_geometry() {
    use kkrieger_oxide::game::beta_monster::HitBox;
    let mut words = [0; 26];
    words[20] = 2f32.to_bits();
    words[21] = 4f32.to_bits();
    words[22] = 6f32.to_bits();
    let hit = HitBox::decode(&words).unwrap();
    let hull = hit.collision_hull();
    assert_eq!(hull.count, 6);
    assert_eq!(hull.mode, 1);
    assert!(!hull.only_for_shot);
    assert_eq!(hull.center, [0., 0., 0., 1.]);
    assert!(hull.planes[..6].iter().flatten().all(|v| v.is_finite()));
}
