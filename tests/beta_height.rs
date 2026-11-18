use kkrieger_oxide::game::beta_monster::actor::Actor;
#[test]
fn vertical_correction_matches_released_ai() {
    let bytes = include_bytes!("fixtures/beta-height.bin");
    assert_eq!(bytes.len(), 6148);
    for record in bytes[4..].as_chunks::<24>().0 {
        let values: [f32; 6] = std::array::from_fn(|i| {
            f32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let [gravity, height, gain, clearance, vertical, expected] = values;
        assert_eq!(
            Actor::correct_height(vertical, clearance, gravity, height, gain).to_bits(),
            expected.to_bits()
        );
    }
}
