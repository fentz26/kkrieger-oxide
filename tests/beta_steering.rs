use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn steering_scale_matches_released_ai() {
    let mut words = include_bytes!("fixtures/beta-steering.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let distance = f32::from_bits(words.next().unwrap());
        let mut data = [0; 26];
        let speed = words.next().unwrap();
        data[6] = 1f32.to_bits();
        data[8] = words.next().unwrap();
        data[18] = words.next().unwrap();
        data[19] = words.next().unwrap();
        let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
        actor.initial.copied.parameters[0] = speed;
        assert_eq!(
            actor.steering_scale(distance).to_bits(),
            words.next().unwrap(),
            "case {case}"
        );
    }
    assert!(words.next().is_none());
}
