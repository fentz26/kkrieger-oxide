use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn steering_displacement_matches_released_ai() {
    let bytes = include_bytes!("fixtures/beta-displacement.bin");
    assert_eq!(bytes.len(), 9220);
    for (case, record) in bytes[4..].as_chunks::<36>().0.iter().enumerate() {
        let words: [u32; 9] = std::array::from_fn(|i| {
            u32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let mut data = [0; 26];
        data[8] = words[2];
        data[18] = words[3];
        data[19] = words[4];
        let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
        actor.initial.copied.parameters[0] = words[1];
        actor.body.particles[0].vectors[0] = [1000., 2., -1000., 1.];
        let delta = actor.steering_displacement(f32::from_bits(words[0]), [0.6, 0., 0.8, 0.]);
        assert_eq!(delta.map(f32::to_bits), words[5..], "case {case}");
    }
}
