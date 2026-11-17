use kkrieger_oxide::{
    effects::Effects,
    game::{
        player::Sound,
        simulation::{Audio, Listener, Tick},
    },
    vfx::{Data, Sample},
};
fn sound(slot: i32) -> Sound {
    Sound {
        slot,
        volume: 1.0,
        near_distance: 0.2,
        far_distance: 160.0,
    }
}
#[test]
fn tick_routes_sound_positions_and_commits_last_listener_once() {
    let samples = [Sample {
        slot: 7,
        buffers: 4,
        data: Data::Mono(vec![16384; 441].into_boxed_slice()),
    }];
    let mut effects = Effects::new(&samples);
    let mut tick = Tick::default();
    tick.sounds.push(Audio {
        sound: sound(7),
        position: [1.0, 2.0, 3.0, 1.0],
    });
    for x in [4.0, 5.0] {
        tick.listeners.push(Listener {
            position: [x, 0.0, 0.0, 1.0],
            velocity: [2.0, 3.0, 4.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            forward: [0.0, 0.0, 1.0, 0.0],
        });
    }
    effects.tick(&tick, 100);
    assert_eq!(effects.pool.sound_time, 100);
    assert_eq!(effects.pool.listener().position, [5.0, 0.0, 0.0]);
    assert_eq!(effects.pool.listener().velocity, [2.0, 3.0, 4.0]);
    let voice = &effects.pool.sample(7).unwrap().voices[1];
    assert_eq!(voice.started, 1);
    let spatial = voice.spatial.unwrap();
    assert_eq!(spatial.position, [1.0, 2.0, 3.0]);
    assert_eq!(spatial.velocity, [0.0; 3]);
    assert_eq!((spatial.near, spatial.far), (0.2, 160.0));
    assert_eq!(effects.play(sound(7), [9.0, 8.0, 7.0, 1.0]), Some(2));
    assert_eq!(effects.pool.sample(7).unwrap().voices[2].started, 100);
    assert!(effects.play(sound(-1), [0.0; 4]).is_none());
    assert!(effects.play(sound(256), [0.0; 4]).is_none());
    assert!(effects.play(sound(6), [0.0; 4]).is_none());
}
#[test]
fn authored_bank_sound_request_reaches_mixed_pcm() {
    let doc =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let samples = kkrieger_oxide::vfx::Bank::parse(doc.samples)
        .unwrap()
        .render(123)
        .unwrap();
    let mut effects = Effects::new(&samples);
    assert_eq!(effects.play(sound(0), [0.0; 4]), Some(1));
    let mut output = vec![[0.0; 2]; 8192];
    effects.mix(&mut output);
    assert!(output.iter().flatten().all(|v| v.is_finite()));
    assert!(output.iter().flatten().any(|v| v.abs() > 0.01));
}
