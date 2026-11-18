use kkrieger_oxide::game::beta_monster::dynamic::motion_delta;
#[test]
fn dynamic_motion_delta_matches_original_inverse_and_multiply() {
    let bytes = include_bytes!("fixtures/beta-motion-delta.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let confirmed =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let current =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        for (field, v) in motion_delta(confirmed, current)
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
