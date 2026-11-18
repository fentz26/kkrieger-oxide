use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn event_attack_phases_match_released_ai() {
    let bytes = include_bytes!("fixtures/beta-attack-modulation.bin");
    assert_eq!(bytes.len(), 2052);
    for record in bytes[4..].as_chunks::<16>().0 {
        let words: [u32; 4] = std::array::from_fn(|i| {
            u32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let mut actor = Actor::spawn(&[0; 26], identity(), 0, 1, |_| None).unwrap();
        actor.cooldowns = [f32::from_bits(words[0]), f32::from_bits(words[1])];
        actor.update_attack_modulation();
        assert_eq!(actor.initial.event_words[13..15], words[2..]);
    }
}
