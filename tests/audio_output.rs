#![cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use kkrieger_oxide::audio_output::Queue;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[test]
#[ignore = "opens the native default audio output; run explicitly"]
fn native_queue_refills_and_stops_before_releasing_callback() {
    struct Lifetime(Arc<AtomicBool>);
    impl Drop for Lifetime {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let released = Arc::new(AtomicBool::new(false));
    let lifetime = Lifetime(released.clone());
    let callback_calls = calls.clone();
    let queue = Queue::open(move |frames| {
        let _keep_alive = &lifetime;
        assert_eq!(frames.len(), 1024);
        frames.fill([0.0; 2]);
        callback_calls.fetch_add(1, Ordering::SeqCst);
    })
    .expect("open native output");
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let mut queue = queue;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) <= 3 && std::time::Instant::now() < deadline {
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        queue.pump().expect("refill native output");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        calls.load(Ordering::SeqCst) > 3,
        "only priming callbacks ran"
    );
    assert_eq!(queue.status(), 0);
    drop(queue);
    assert!(released.load(Ordering::SeqCst));
    let stopped = calls.load(Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(calls.load(Ordering::SeqCst), stopped);
}

#[test]
#[ignore = "opens native audio output; run explicitly"]
fn renderer_panic_is_contained_and_releases_state() {
    struct Lifetime(Arc<AtomicBool>);
    impl Drop for Lifetime {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let released = Arc::new(AtomicBool::new(false));
    let lifetime = Lifetime(released.clone());
    let result = Queue::open(move |_| {
        let _keep_alive = &lifetime;
        panic!("test renderer failure");
    });
    assert!(matches!(
        result,
        Err(kkrieger_oxide::audio_output::Error(i32::MIN))
    ));
    assert!(released.load(Ordering::SeqCst));
}

#[test]
#[ignore = "opens native output with the authored music; run explicitly"]
fn authored_music_worker_owns_song_until_synchronous_shutdown() {
    let doc =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let output = kkrieger_oxide::music::output::Output::open(doc.song).unwrap();
    output.check().unwrap();
    drop(output);
}

#[test]
#[ignore = "plays a short authored effect through native output; run explicitly"]
fn authored_effect_bank_and_game_requests_reach_native_audio_worker() {
    use kkrieger_oxide::{
        game::{player::Sound, simulation::Tick},
        music::output::Output,
    };
    let doc =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let output = Output::open_with_effects(doc.song, doc.samples).unwrap();
    let mut tick = Tick::default();
    tick.listeners
        .push(kkrieger_oxide::game::simulation::Listener {
            position: [0.0; 4],
            velocity: [90.0, 0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0, 0.0],
            forward: [0.0, 0.0, 1.0, 0.0],
        });
    output.tick(tick, 100).unwrap();
    output
        .play(
            Sound {
                slot: 0,
                volume: 1.0,
                near_distance: 0.2,
                far_distance: 160.0,
            },
            [1.0, 0.0, 0.0, 1.0],
        )
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    output.check().unwrap();
    drop(output);
}

#[test]
#[ignore = "opens native output to exercise command backpressure; run explicitly"]
fn native_audio_accepts_tick_larger_than_request_capacity() {
    use kkrieger_oxide::{
        game::{
            player::Sound,
            simulation::{Audio, Tick},
        },
        music::output::Output,
    };
    let doc =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let output = Output::open(doc.song).unwrap();
    let mut tick = Tick::default();
    for _ in 0..1500 {
        tick.sounds.push(Audio {
            sound: Sound {
                slot: 255,
                volume: 1.0,
                near_distance: 0.2,
                far_distance: 160.0,
            },
            position: [0.0; 4],
        });
    }
    output.tick(tick, 100).unwrap();
    output.pause_music().unwrap();
    output.restart_music().unwrap();
    output.check().unwrap();
    drop(output);
}
