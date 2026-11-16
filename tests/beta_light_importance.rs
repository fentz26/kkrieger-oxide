use kkrieger_oxide::scene::light::beta_importance;
#[test]
fn released_beta_importance_and_fade_match_downward_single_precision() {
    let mut bytes: &[u8] = include_bytes!("fixtures/beta-light-importance.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    fn real(bytes: &mut &[u8]) -> f32 {
        f32::from_bits(word(bytes))
    }
    assert_eq!(word(&mut bytes), 512);
    for case in 0..512 {
        let result = beta_importance(real(&mut bytes), real(&mut bytes), real(&mut bytes));
        assert_eq!(
            result.importance.to_bits(),
            word(&mut bytes),
            "case {case} importance"
        );
        assert_eq!(
            result.amplify.to_bits(),
            word(&mut bytes),
            "case {case} amplification"
        );
    }
    assert!(bytes.is_empty());
}
