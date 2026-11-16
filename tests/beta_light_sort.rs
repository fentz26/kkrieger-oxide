use kkrieger_oxide::scene::light::beta_light_order;
#[test]
fn released_beta_light_sort_preserves_ties_and_clamps_count_after_sorting() {
    let mut bytes: &[u8] = include_bytes!("fixtures/beta-light-sort.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    assert_eq!(word(&mut bytes), 128);
    for case in 0..128 {
        let count = word(&mut bytes) as usize;
        let importance: Vec<_> = (0..count)
            .map(|_| f32::from_bits(word(&mut bytes)))
            .collect();
        let selected = beta_light_order(&importance);
        let count = word(&mut bytes) as usize;
        assert_eq!(selected.len(), count, "case {case}");
        for id in selected {
            assert_eq!(id, word(&mut bytes) as usize, "case {case}");
        }
    }
    assert!(bytes.is_empty());
}
