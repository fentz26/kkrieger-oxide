use kkrieger_oxide::{
    sample_playback::{Pool, Spatial},
    vfx::{Data, Sample},
};

fn sample() -> Sample {
    Sample {
        slot: 7,
        buffers: 4,
        data: Data::Mono(vec![0; 441].into_boxed_slice()),
    }
}

#[test]
fn rotation_steals_oldest_and_rejects_stale_spatial_tokens() {
    let sample = sample();
    let mut pool = Pool::new();
    assert!(pool.add(&sample));
    assert!(!pool.add(&sample));
    pool.commit(100);
    for token in 1..=5 {
        assert_eq!(pool.play(7, 1.0, 0.0), Some(token));
        let voice = &pool.sample(7).unwrap().voices[token as usize % 4];
        assert_eq!(voice.started, 100);
        assert_eq!(voice.position, 0);
    }
    assert!(!pool.spatial(7, 1, Spatial::default()));
    assert!(pool.spatial(7, 2, Spatial::default()));
    assert!(pool.spatial(7, 5, Spatial::default()));
    assert!(pool.sample(7).unwrap().voices[1].spatial.is_some());
}

#[test]
fn expiry_uses_previous_commit_clock_and_strict_comparison() {
    let sample = sample(); // 10 ms PCM + original 10 ms margin
    let mut pool = Pool::new();
    pool.add(&sample);
    pool.commit(100);
    let token = pool.play(7, 0.5, 20.0).unwrap();
    assert!(pool.spatial(7, token, Spatial::default()));
    pool.commit(120);
    pool.commit(121); // compares 120, equality does not expire
    assert!(pool.sample(7).unwrap().voices[1].spatial.is_some());
    pool.commit(122); // compares 121
    assert_eq!(pool.sample(7).unwrap().voices[1].started, 0);
    assert!(pool.sample(7).unwrap().voices[1].spatial.is_none());
    assert!(!pool.spatial(7, token, Spatial::default()));
}

#[test]
fn zero_clock_marker_volume_pan_and_registration_bounds() {
    let sample = sample();
    let mut pool = Pool::new();
    pool.add(&sample);
    let token = pool.play(7, 0.5, 200.0).unwrap();
    assert!(!pool.spatial(7, token, Spatial::default()));
    let voice = &pool.sample(7).unwrap().voices[1];
    assert_eq!(voice.volume, -1386); // original natural logarithm
    assert_eq!(voice.pan, 0); // mono 3D buffers ignore pan
    let stereo = Sample {
        slot: 8,
        buffers: 1,
        data: Data::Stereo(vec![[0, 0]; 1].into_boxed_slice()),
    };
    pool.add(&stereo);
    pool.play(8, 2.0, -200.0);
    let voice = &pool.sample(8).unwrap().voices[0];
    assert_eq!((voice.volume, voice.pan), (0, -10000));
    assert!(pool.play(9, 1.0, 0.0).is_none());
    assert!(pool.play(7, f32::NAN, 0.0).is_none());
    let invalid = Sample {
        slot: 9,
        buffers: 9,
        data: Data::Mono(vec![0].into_boxed_slice()),
    };
    assert!(!pool.add(&invalid));
}

#[test]
fn every_supported_buffer_count_retains_parameters_until_commit_or_removal() {
    for count in 1..=8 {
        let sample = Sample {
            slot: 255,
            buffers: count,
            data: Data::Mono(vec![0; 441].into_boxed_slice()),
        };
        let mut pool = Pool::new();
        pool.add(&sample);
        pool.commit(1);
        for token in 1..=32 {
            assert_eq!(pool.play(255, 1.0, 0.0), Some(token));
            let registered = pool.sample(255).unwrap();
            assert_eq!(registered.last, token as usize % usize::from(count));
            assert!(!pool.spatial(255, token - i32::from(count), Spatial::default()));
            assert!(pool.spatial(255, token, Spatial::default()));
        }
        for token in 33..=32 + i32::from(count) {
            pool.play(255, 1.0, 0.0);
            assert!(
                pool.sample(255).unwrap().voices[token as usize % usize::from(count)]
                    .spatial
                    .is_some()
            );
        }
        pool.remove(255);
        assert!(pool.play(255, 1.0, 0.0).is_none());
        assert!(pool.add(&sample));
        assert_eq!(pool.play(255, 1.0, 0.0), Some(1));
    }
}

#[test]
fn mixing_is_additive_one_shot_and_independent_of_callback_size() {
    let sample = Sample {
        slot: 1,
        buffers: 2,
        data: Data::Stereo(vec![[32767, -32768], [16384, -16384], [1, -1]].into_boxed_slice()),
    };
    let run = |split: bool| {
        let mut pool = Pool::new();
        pool.add(&sample);
        let mut silence = [[0.0; 2]; 1];
        pool.mix(&mut silence, |_| [1.0; 2]);
        assert_eq!(silence, [[0.0; 2]]);
        pool.play(1, 1.0, 0.0);
        pool.play(1, 1.0, 0.0);
        let mut output = [[0.25; 2]; 5];
        if split {
            for chunk in output.chunks_mut(1) {
                pool.mix(chunk, |_| [1.0; 2]);
            }
        } else {
            pool.mix(&mut output, |_| [1.0; 2]);
        }
        assert!(!pool.sample(1).unwrap().voices[0].playing);
        assert!(!pool.sample(1).unwrap().voices[1].playing);
        output
    };
    let output = run(false);
    assert_eq!(output, run(true));
    assert_eq!(output[0], [0.25 + 2.0 * 32767.0 / 32768.0, -1.75]);
    assert_eq!(output[1], [1.25, -0.75]);
    assert_eq!(output[3..], [[0.25; 2]; 2]);
}

#[test]
fn stereo_pan_and_mono_spatial_gains_are_applied_per_voice() {
    let stereo = Sample {
        slot: 1,
        buffers: 1,
        data: Data::Stereo(vec![[16384, 16384]].into_boxed_slice()),
    };
    let mono = Sample {
        slot: 2,
        buffers: 1,
        data: Data::Mono(vec![16384].into_boxed_slice()),
    };
    let mut pool = Pool::new();
    pool.add(&stereo);
    pool.add(&mono);
    pool.commit(1);
    pool.play(1, 1.0, -100.0);
    let token = pool.play(2, 1.0, 0.0).unwrap();
    pool.spatial(2, token, Spatial::default());
    let mut output = [[0.0; 2]];
    pool.mix(&mut output, |_| [0.25, 0.75]);
    assert_eq!(output, [[0.625, 0.375]]);
}

#[test]
fn out_of_range_directsound_volume_keeps_previous_buffer_setting() {
    let sample = Sample {
        slot: 1,
        buffers: 1,
        data: Data::Mono(vec![16384].into_boxed_slice()),
    };
    let mut pool = Pool::new();
    pool.add(&sample);
    pool.play(1, 0.5, 0.0);
    assert_eq!(pool.sample(1).unwrap().voices[0].volume, -1386);
    pool.play(1, 0.0, 0.0);
    assert_eq!(pool.sample(1).unwrap().voices[0].volume, -1386);
}

#[test]
fn listener_updates_are_deferred_and_last_update_wins() {
    use kkrieger_oxide::sample_playback::Listener;
    let mut pool = Pool::new();
    let initial = *pool.listener();
    let first = Listener {
        position: [10.0, 0.0, 0.0],
        ..initial
    };
    let last = Listener {
        position: [20.0, 0.0, 0.0],
        velocity: [1.0, 2.0, 3.0],
        ..initial
    };
    assert!(pool.set_listener(first));
    assert!(pool.set_listener(last));
    assert_eq!(*pool.listener(), initial);
    pool.commit(100);
    assert_eq!(*pool.listener(), last);
    let invalid = Listener {
        rolloff: f32::NAN,
        ..initial
    };
    assert!(!pool.set_listener(invalid));
    pool.commit(200);
    assert_eq!(*pool.listener(), last);
}

#[test]
fn distance_gain_saturates_at_near_and_far_without_muting() {
    use kkrieger_oxide::sample_playback::Listener;
    let listener = Listener::default();
    let mut sound = Spatial {
        near: 2.0,
        far: 8.0,
        ..Spatial::default()
    };
    assert_eq!(listener.distance_gain(&sound), 1.0);
    sound.position = [0.0, 0.0, 2.0];
    assert_eq!(listener.distance_gain(&sound), 1.0);
    sound.position = [0.0, 0.0, 4.0];
    assert_eq!(listener.distance_gain(&sound), 0.5);
    sound.position = [0.0, 0.0, 100.0];
    assert_eq!(listener.distance_gain(&sound), 0.25);
    let listener = Listener {
        rolloff: 0.0,
        ..listener
    };
    assert_eq!(listener.distance_gain(&sound), 1.0);
    let listener = Listener {
        position: sound.position,
        ..listener
    };
    assert_eq!(listener.distance_gain(&sound), 1.0);
}

#[test]
fn invalid_spatial_updates_leave_playing_voice_unchanged() {
    let sample = sample();
    let mut pool = Pool::new();
    pool.add(&sample);
    pool.commit(1);
    let token = pool.play(7, 1.0, 0.0).unwrap();
    let original = Spatial::default();
    assert!(pool.spatial(7, token, original));
    for invalid in [
        Spatial {
            near: 0.0,
            ..original
        },
        Spatial {
            near: 10.0,
            far: 1.0,
            ..original
        },
        Spatial {
            position: [f32::NAN, 0.0, 0.0],
            ..original
        },
        Spatial {
            velocity: [f32::INFINITY, 0.0, 0.0],
            ..original
        },
    ] {
        assert!(!pool.spatial(7, token, invalid));
        assert_eq!(pool.sample(7).unwrap().voices[1].spatial, Some(original));
    }
}
