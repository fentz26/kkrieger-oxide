use kkrieger_oxide::game::beta_monster::hit::finish_hit;
#[test]
fn hit_event_state_matches_released_no_effect_path() {
    let bytes = include_bytes!("fixtures/beta-hit-state.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let present = words.next().unwrap() != 0;
        let reflect = words.next().unwrap() != 0;
        let monster = words.next().unwrap() != 0;
        let time = words.next().unwrap() as i32;
        let mut event: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        finish_hit(
            if present { Some(&mut event) } else { None },
            reflect,
            monster,
            time,
        );
        let expected: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(event, expected, "case {case}");
    }
    assert!(words.next().is_none());
}
