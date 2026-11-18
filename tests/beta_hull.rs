use kkrieger_oxide::game::beta_monster::hull;
#[test]
fn hull_queries_match_released_plane_routines() {
    let bytes = include_bytes!("fixtures/beta-hull.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(512));
    for case in 0..512 {
        let count = words.next().unwrap() as usize;
        let point = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let direction = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let radius = f32::from_bits(words.next().unwrap());
        let planes: [[f32; 4]; 12] =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let planes = &planes[..count];
        assert_eq!(
            hull::outside_mask(planes, point, radius),
            words.next().unwrap(),
            "case {case} mask"
        );
        for hit in [
            hull::intersect_out(planes, point, direction),
            hull::intersect_in(planes, point, direction),
        ] {
            let expected = words.next().unwrap() != 0;
            let fields: [u32; 5] = std::array::from_fn(|_| words.next().unwrap());
            assert_eq!(hit.is_some(), expected, "case {case} hit");
            if let Some(hit) = hit {
                let actual = [
                    hit.plane[0].to_bits(),
                    hit.plane[1].to_bits(),
                    hit.plane[2].to_bits(),
                    hit.plane[3].to_bits(),
                    hit.distance.to_bits(),
                ];
                assert_eq!(actual, fields, "case {case} intersection");
            } else {
                assert_eq!(fields, [0xa5a5a5a5; 5], "case {case} untouched outputs");
            }
        }
    }
    assert!(words.next().is_none());
}
