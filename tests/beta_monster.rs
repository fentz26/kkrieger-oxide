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
