use kkrieger_oxide::game::{
    beta_monster::body::Body,
    beta_player::{Control, Settings},
};
#[test]
fn player_translation_and_landing_match_released_instructions() {
    let bytes = include_bytes!("fixtures/beta-player-control.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let right = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let forward = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let mut next = || f32::from_bits(words.next().unwrap());
        let side = next();
        let ahead = next();
        let speed_up = next();
        let gravity = next();
        let height = next();
        let height_gain = next();
        let clearance = next();
        let off_ground_time = next();
        let slices = words.next().unwrap() as i32;
        let mut control = Control {
            speed_up,
            on_ground: true,
            off_ground_time,
            step_timer: 1.25,
            step_foot: 7,
        };
        let mut body = Body::new(position, None, 1);
        let before = body.clone();
        let sound = control.prepare(
            &mut body,
            &Settings {
                right,
                forward,
                side,
                ahead,
                gravity,
                height,
                height_gain,
                slices,
            },
            clearance,
        );
        for (index, particle) in body.particles.iter().enumerate() {
            let expected: Vec<_> = words.by_ref().take(32).collect();
            for (axis, value) in particle.vectors[0].iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    expected[16 + axis],
                    "case {case} particle {index} axis {axis}"
                );
            }
            assert_eq!(particle.flags, before.particles[index].flags);
            assert_eq!(particle.vectors[1..], before.particles[index].vectors[1..]);
            assert_eq!(particle.force, before.particles[index].force);
        }
        assert_eq!(body.constraints, before.constraints);
        assert_eq!(control.speed_up.to_bits(), words.next().unwrap());
        assert_eq!(u32::from(control.on_ground), words.next().unwrap());
        assert_eq!(control.step_timer.to_bits(), words.next().unwrap());
        assert_eq!(control.step_foot as u32, words.next().unwrap());
        assert_eq!(
            control.off_ground_time.to_bits(),
            words.next().unwrap(),
            "time case {case}"
        );
        assert_eq!(
            u32::from(sound.is_some()),
            words.next().unwrap(),
            "sound case {case}"
        );
        if let Some(sound) = sound {
            assert_eq!(sound.slot, 5);
            assert_eq!(sound.volume, 1.);
        }
    }
    assert!(words.next().is_none());
}
