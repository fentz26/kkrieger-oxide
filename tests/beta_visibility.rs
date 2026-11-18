use kkrieger_oxide::{game::beta_monster::actor::Actor, mesh::identity};
#[test]
fn activation_visibility_gate_matches_released_ai() {
    let bytes = include_bytes!("fixtures/beta-visibility.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(144));
    for case in 0..144 {
        let active = words.next().unwrap() != 0;
        let occluded = words.next().unwrap() != 0;
        let present = words.next().unwrap() != 0;
        let distance = f32::from_bits(words.next().unwrap());
        let radius = words.next().unwrap();
        let mut data = [0; 26];
        data[10] = 20;
        data[7] = radius;
        for v in &mut data[20..23] {
            *v = 1f32.to_bits();
        }
        let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
        actor.active = active;
        let mut casts = 0;
        actor.update_visibility(
            distance,
            || {
                casts += 1;
                occluded
            },
            |_| present.then_some(0),
        );
        assert_eq!(casts, words.next().unwrap(), "case {case} casts");
        assert_eq!(
            u32::from(actor.active),
            words.next().unwrap(),
            "case {case} activation"
        );
        for p in &actor.body.particles {
            assert_eq!(
                if p.cell.is_some() { 0x3006000 } else { 0 },
                words.next().unwrap(),
                "case {case} body cell"
            );
        }
    }
    assert!(words.next().is_none());
}
