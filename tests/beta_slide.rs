use kkrieger_oxide::game::beta_monster::collision::Travel;
#[test]
fn travel_and_first_contact_match_released_math() {
    let bytes = include_bytes!("fixtures/beta-slide.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let origin = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let target = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let plane = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let distance = f32::from_bits(words.next().unwrap());
        let active = words.next().unwrap() != 0;
        let travel = Travel::new(origin, target);
        assert_eq!(travel.is_some(), active, "case {case}");
        let expected: [u32; 19] = std::array::from_fn(|_| words.next().unwrap());
        if let Some(travel) = travel {
            let slide = travel.first_contact(plane, distance);
            let actual = travel
                .direction
                .into_iter()
                .chain([travel.inverse_length])
                .chain(slide.impact)
                .chain(slide.travel.origin)
                .chain(slide.travel.target)
                .chain(slide.travel.direction);
            for (field, (actual, expected)) in actual.zip(expected).enumerate() {
                assert_eq!(actual.to_bits(), expected, "case {case} field {field}");
            }
        } else {
            assert_eq!(expected, [0; 19]);
        }
    }
    assert!(words.next().is_none());
}
