#![cfg(target_os = "macos")]
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
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) <= 3 && std::time::Instant::now() < deadline {
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
#[ignore = "opens a native Audio Queue; run explicitly"]
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
