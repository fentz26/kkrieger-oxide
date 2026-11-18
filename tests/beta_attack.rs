use kkrieger_oxide::{
    game::beta_monster::actor::{Actor, Attack},
    mesh::identity,
};
#[test]
fn attack_selection_and_events_match_released_ai() {
    let mut words = include_bytes!("fixtures/beta-attack.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(384));
    for case in 0..384 {
        let distance = f32::from_bits(words.next().unwrap());
        let timers = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let hits = words.next().unwrap();
        let op = words.next().unwrap();
        let blocked = words.next().unwrap() != 0;
        let mut data = [0; 26];
        data[12] = hits;
        data[14] = hits;
        let mut matrix = identity();
        matrix[3] = [5., 6., 7., 1.];
        let mut actor = Actor::spawn(&data, matrix, op, 0x3001000, |_| None).unwrap();
        actor.active = true;
        actor.cooldowns = timers;
        for i in 0..8 {
            actor.initial.event_words[17 + i] = 0x3f800000 + i as u32;
        }
        let mut casts = 0;
        let attack = actor.select_attack(distance, [2., 3., 4., 1.], |start, end| {
            casts += 1;
            assert_eq!(start, [2., 4., 4., 1.]);
            assert_eq!(end, [5., 7., 7., 1.]);
            blocked
        });
        let damage = words.next().unwrap();
        let expected_casts = words.next().unwrap();
        let emitted = words.next().unwrap() != 0;
        assert_eq!(casts, expected_casts, "case {case} casts");
        for v in actor.cooldowns {
            assert_eq!(v.to_bits(), words.next().unwrap(), "case {case} cooldown");
        }
        if emitted {
            let expected = std::array::from_fn(|_| words.next().unwrap());
            assert_eq!(attack, Some(Attack::Ranged(expected)), "case {case}");
        } else if damage != 0 {
            assert_eq!(attack, Some(Attack::Melee(damage as i32)), "case {case}");
        } else {
            assert_eq!(attack, None, "case {case}");
        }
    }
    assert!(words.next().is_none());
}
