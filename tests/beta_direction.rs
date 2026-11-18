use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn player_relative_direction_matches_released_ai() {
    let bytes = include_bytes!("fixtures/beta-direction.bin");
    assert_eq!(bytes.len(), 6660);
    for (case, record) in bytes[4..].as_chunks::<52>().0.iter().enumerate() {
        let words: [u32; 13] = std::array::from_fn(|i| {
            u32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let mut actor = Actor::spawn(&[0; 26], identity(), 0, 1, |_| None).unwrap();
        actor.body.particles[0].vectors[0] = std::array::from_fn(|i| f32::from_bits(words[i]));
        let (direction, distance) =
            actor.player_direction(std::array::from_fn(|i| f32::from_bits(words[4 + i])));
        assert_eq!(direction.map(f32::to_bits), words[8..12], "case {case}");
        assert_eq!(distance.to_bits(), words[12], "case {case}");
    }
}
