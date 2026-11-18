use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn event_position_and_facing_match_released_ai() {
    let bytes = include_bytes!("fixtures/beta-facing.bin");
    assert_eq!(bytes.len(), 8196);
    for record in bytes[4..].as_chunks::<64>().0 {
        let words: [u32; 16] = std::array::from_fn(|i| {
            u32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let mut actor = Actor::spawn(&[0; 26], identity(), 0, 1, |_| None).unwrap();
        actor.body.particles[0].vectors[0] = std::array::from_fn(|i| f32::from_bits(words[i]));
        actor.update_event_pose(std::array::from_fn(|i| f32::from_bits(words[4 + i])));
        assert_eq!(actor.initial.event_words[17..25], words[8..]);
    }
}
