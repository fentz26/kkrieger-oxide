use kkrieger_oxide::game::beta_monster::hit::{Hit, apply_hit};
#[test]
fn effect_and_shot_state_match_released_hit_handler() {
    let bytes = include_bytes!("fixtures/beta-hit-effect.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let kind = words.next().unwrap() as i32;
        let present = words.next().unwrap() != 0;
        let reflect = words.next().unwrap() != 0;
        let monster = words.next().unwrap() != 0;
        let time = words.next().unwrap() as i32;
        let operator = words.next().unwrap();
        let mut event: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let normal = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let before = event;
        let hit = Hit {
            kind,
            reflect,
            monster,
            time,
            operator,
            position,
            normal,
        };
        let result = apply_hit(if present { Some(&mut event) } else { None }, hit);
        let expected: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(event, expected, "case {case} shot");
        let mut scene = kkrieger_oxide::scene::event::Event::from_released_words(
            &before,
            Some(7),
            Some(9),
            Some(11),
        );
        scene.interval = [0.25, 0.75];
        let original = scene.clone();
        let mut captured = scene.released_hit_state();
        let bridged = apply_hit(if present { Some(&mut captured) } else { None }, hit);
        scene.apply_released_hit_state(&captured);
        for field in [2, 3, 5, 10, 12] {
            assert_eq!(
                scene.released_hit_state()[field],
                expected[field],
                "case {case} field {field}"
            );
        }
        assert_eq!(bridged, result);
        let vectors = |event: &kkrieger_oxide::scene::event::Event| {
            event
                .matrix
                .into_iter()
                .chain([event.scale, event.rotate, event.translate])
                .flatten()
                .map(f32::to_bits)
                .collect::<Vec<_>>()
        };
        assert_eq!(vectors(&scene), vectors(&original));
        assert_eq!(
            (
                scene.operator,
                scene.spline,
                scene.monster,
                scene.color,
                scene.cull_distance,
                scene.velocity.to_bits(),
                scene.interval.map(f32::to_bits)
            ),
            (
                original.operator,
                original.spline,
                original.monster,
                original.color,
                original.cull_distance,
                original.velocity.to_bits(),
                original.interval.map(f32::to_bits)
            )
        );
        let created = words.next().unwrap() != 0;
        let effect: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(result.is_some(), created, "case {case} creation");
        assert_eq!(result.unwrap_or([0; 45]), effect, "case {case} effect");
    }
    assert!(words.next().is_none());
}
