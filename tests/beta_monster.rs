use kkrieger_oxide::game::beta_monster::InitialCopy;
#[test]
fn released_field_copy_matches_x86_handler() {
    let bytes = include_bytes!("fixtures/beta-monster-copy.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    let count = words.next().unwrap();
    assert_eq!(count, 256);
    for case in 0..count {
        let input: [u32; 26] = std::array::from_fn(|_| words.next().unwrap());
        let decoded = InitialCopy::decode(&input).unwrap();
        for vector in decoded.vectors {
            for value in vector {
                assert_eq!(value.to_bits(), words.next().unwrap(), "case {case}");
            }
        }
        for value in decoded.parameters {
            assert_eq!(value, words.next().unwrap(), "case {case}");
        }
    }
    assert!(words.next().is_none());
}
#[test]
fn released_record_requires_all_twenty_six_words() {
    for length in 0..26 {
        assert!(InitialCopy::decode(&vec![0; length]).is_err());
    }
    assert!(InitialCopy::decode(&[0; 26]).is_ok());
    assert!(InitialCopy::decode(&[0; 29]).is_ok());
}
#[test]
fn released_hit_box_matches_actual_math_callees() {
    use kkrieger_oxide::game::beta_monster::HitBox;
    let bytes = include_bytes!("fixtures/beta-monster-geometry.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let input: [u32; 26] = std::array::from_fn(|_| words.next().unwrap());
        let decoded = HitBox::decode(&input).unwrap();
        for (field, value) in decoded
            .matrix
            .into_iter()
            .flatten()
            .chain(decoded.scale)
            .chain(decoded.vertices().into_iter().flatten())
            .enumerate()
        {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case {case} field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}
#[test]
fn released_spawn_prefix_matches_original_event_and_spline_routines() {
    use kkrieger_oxide::game::beta_monster::InitialState;
    let bytes = include_bytes!("fixtures/beta-monster-spawn.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let input: [u32; 26] = std::array::from_fn(|_| words.next().unwrap());
        let parent =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let spline = words.next().unwrap();
        let actor = words.next().unwrap();
        let state = InitialState::decode(&input, parent, spline, actor).unwrap();
        let actual = state
            .matrix
            .into_iter()
            .flatten()
            .map(f32::to_bits)
            .chain(state.copied.parameters)
            .chain(state.event_words)
            .chain(state.death_event_words)
            .chain([state.attack_operator, state.life as u32])
            .chain(state.cell_query_position.map(f32::to_bits))
            .chain([state.collider_type]);
        for (field, value) in actual.enumerate() {
            assert_eq!(value, words.next().unwrap(), "case {case} field {field}");
        }
    }
    assert!(words.next().is_none());
}
