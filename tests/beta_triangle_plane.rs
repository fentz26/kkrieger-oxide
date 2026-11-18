use kkrieger_oxide::game::beta_monster::hull::triangle_plane;
#[test]
fn collision_planes_match_released_triangle_constructor() {
    let bytes = include_bytes!("fixtures/beta-triangle-plane.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let a = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let b = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let c = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let hit = words.next().unwrap() != 0;
        let expected: [u32; 4] = std::array::from_fn(|_| words.next().unwrap());
        let plane = triangle_plane(a, b, c);
        assert_eq!(plane.is_some(), hit, "case {case}");
        if let Some(plane) = plane {
            assert_eq!(plane.map(f32::to_bits), expected, "case {case} plane");
        } else {
            assert_eq!(expected, [0xa5a5a5a5; 4], "case {case} untouched result");
        }
    }
    assert!(words.next().is_none());
}
