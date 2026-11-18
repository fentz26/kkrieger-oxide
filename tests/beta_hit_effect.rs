use kkrieger_oxide::game::beta_monster::hit::{Hit, apply_hit};
#[test]
fn effect_and_shot_state_match_released_hit_handler() {
    let bytes = include_bytes!("fixtures/beta-hit-effect.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let kind = words.next().unwrap() as i32;
        let present = words.next().unwrap() != 0;
        let reflect = words.next().unwrap() != 0;
        let monster = words.next().unwrap() != 0;
        let time = words.next().unwrap() as i32;
        let operator = words.next().unwrap();
        let mut event: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let normal = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let result = apply_hit(
            if present { Some(&mut event) } else { None },
            Hit {
                kind,
                reflect,
                monster,
                time,
                operator,
                position,
                normal,
            },
        );
        let expected: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(event, expected, "case {case} shot");
        let created = words.next().unwrap() != 0;
        let effect: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(result.is_some(), created, "case {case} creation");
        assert_eq!(result.unwrap_or([0; 45]), effect, "case {case} effect");
    }
    assert!(words.next().is_none());
}
