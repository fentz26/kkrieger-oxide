use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn damage_activation_and_switches_match_original_hit() {
    let bytes = include_bytes!("fixtures/beta-actor-damage.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let life = words.next().unwrap();
        let armor = words.next().unwrap();
        let hits = words.next().unwrap() as i32;
        let kill = words.next().unwrap();
        let active = words.next().unwrap() != 0;
        let present = words.next().unwrap() != 0;
        let switch_words: [u32; 64] = std::array::from_fn(|_| words.next().unwrap());
        let mut switches: [u8; 256] =
            std::array::from_fn(|i| switch_words[i / 4].to_le_bytes()[i % 4]);
        let mut data = [0; 26];
        data[10] = life;
        data[11] = armor;
        data[17] = kill;
        for v in &mut data[20..23] {
            *v = 1f32.to_bits();
        }
        let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
        actor.active = active;
        let mut lookups = 0;
        actor
            .hit(hits, &mut switches, |_| {
                lookups += 1;
                present.then_some(0)
            })
            .unwrap();
        assert_eq!(
            actor.initial.life as u32,
            words.next().unwrap(),
            "case {case} life"
        );
        assert_eq!(
            u32::from(actor.active),
            words.next().unwrap(),
            "case {case} active"
        );
        assert_eq!(
            lookups,
            usize::from((life as i32) > 0 && !active),
            "case {case} lookup order"
        );
        for p in &actor.body.particles {
            let expected = words.next().unwrap();
            assert_eq!(
                if p.cell.is_some() { 0x3006000 } else { 0 },
                expected,
                "case {case} body cell"
            );
        }
        let expected: [u32; 64] = std::array::from_fn(|_| words.next().unwrap());
        let actual: [u32; 64] = std::array::from_fn(|i| {
            u32::from_le_bytes(switches[i * 4..i * 4 + 4].try_into().unwrap())
        });
        assert_eq!(actual, expected, "case {case} switches");
    }
    assert!(words.next().is_none());
}
#[test]
fn invalid_switch_fails_before_activation_or_damage() {
    let mut data = [0; 26];
    data[10] = 20;
    data[17] = 256;
    for v in &mut data[20..23] {
        *v = 1f32.to_bits();
    }
    let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
    let before = actor.body.clone();
    let mut switches = [0; 256];
    assert_eq!(
        actor.hit(30, &mut switches, |_| panic!("unexpected activation")),
        Err("monster kill switch out of range")
    );
    assert_eq!(actor.initial.life, 20);
    assert!(!actor.active);
    assert_eq!(actor.body, before);
    assert_eq!(switches, [0; 256]);
}
