use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn attack_cooldowns_match_released_arithmetic() {
    let mut words = include_bytes!("fixtures/beta-cooldowns.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let mut data = [0; 26];
        data[13] = words.next().unwrap();
        data[15] = words.next().unwrap();
        let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
        actor.active = true;
        actor.cooldowns = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        actor.advance_cooldowns();
        for value in actor.cooldowns {
            assert_eq!(value.to_bits(), words.next().unwrap(), "case {case}");
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn inactive_actor_retains_attack_progress() {
    let mut actor = Actor::spawn(&[0; 26], identity(), 0, 1, |_| None).unwrap();
    actor.cooldowns = [0.25, 0.75];
    actor.advance_cooldowns();
    assert_eq!(actor.cooldowns, [0.25, 0.75]);
}
