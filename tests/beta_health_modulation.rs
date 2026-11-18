use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn life_ratio_and_culling_distance_match_released_ai() {
    let bytes = include_bytes!("fixtures/beta-health-modulation.bin");
    assert_eq!(bytes.len(), 2052);
    for record in bytes[4..].as_chunks::<16>().0 {
        let words: [u32; 4] = std::array::from_fn(|i| {
            u32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let mut data = [0; 26];
        data[10] = words[1];
        let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
        actor.initial.life = words[0] as i32;
        actor.update_health_modulation();
        assert_eq!(actor.initial.event_words[15], words[2]);
        assert_eq!(actor.initial.event_words[44], words[3]);
    }
}
