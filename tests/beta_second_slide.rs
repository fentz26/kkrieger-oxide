use kkrieger_oxide::game::beta_monster::collision::Travel;
#[test]
fn second_contact_matches_released_opposing_projection_and_crease_paths() {
    let bytes = include_bytes!("fixtures/beta-second-slide.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let target = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let direction = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let inverse_length = f32::from_bits(words.next().unwrap());
        let p1 = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let p2 = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let distance = f32::from_bits(words.next().unwrap());
        let old = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let reset = words.next().unwrap() != 0;
        let travel = Travel {
            origin: [0.; 4],
            target,
            direction,
            inverse_length,
        };
        let slide = travel.second_contact(p1, p2, distance, old);
        assert_eq!(slide.reset_position.is_some(), reset, "case {case}");
        let particle_position = slide.reset_position.unwrap_or([0.; 4]);
        let actual = slide
            .travel
            .origin
            .into_iter()
            .chain(slide.travel.target)
            .chain(slide.travel.direction)
            .chain(particle_position)
            .chain(particle_position);
        for (field, value) in actual.enumerate() {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case {case} field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}
