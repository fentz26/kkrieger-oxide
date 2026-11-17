//! Own song storage on a worker while the native queue borrows its synthesizer.
use super::Music;
use crate::{
    audio_output::Queue,
    v2m::{Song, conversion::convert, playback::Synth},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
pub struct Output {
    stop: SyncSender<()>,
    worker: Option<JoinHandle<()>>,
    failed: Arc<AtomicBool>,
}
impl Output {
    pub fn open(song: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let data = convert(song)?;
        let (stop, receiver) = mpsc::sync_channel(1);
        let (ready, started) = mpsc::sync_channel(1);
        let failed = Arc::new(AtomicBool::new(false));
        let thread_failed = Arc::clone(&failed);
        let worker = thread::Builder::new()
            .name("kkrieger-audio".into())
            .spawn(move || {
                if let Err(error) = run(&data, receiver, &ready, &thread_failed) {
                    thread_failed.store(true, Ordering::Release);
                    let _ = ready.send(Err(error.to_string()));
                }
            })?;
        match started.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                worker: Some(worker),
                failed,
            }),
            result => {
                let _ = worker.join();
                Err(match result {
                    Ok(Err(error)) => error,
                    _ => "audio worker stopped during startup".into(),
                }
                .into())
            }
        }
    }
    pub fn check(&self) -> Result<(), &'static str> {
        if self.failed.load(Ordering::Acquire) {
            Err("native music playback failed")
        } else {
            Ok(())
        }
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        let _ = self.stop.try_send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn run(
    data: &[u8],
    stop: Receiver<()>,
    ready: &SyncSender<Result<(), String>>,
    failed: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error>> {
    let song = Song::parse(data)?;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .subsec_nanos();
    let synth = Synth::new(&song, 44100, seed)?;
    let mut music = Music::new(&song, synth)?;
    // This queue never escapes this scope and is synchronously disposed on all
    // return/unwind paths before its borrowed music and song storage expires.
    let queue = unsafe {
        Queue::open_scoped(|output| {
            if failed.load(Ordering::Acquire) || music.render(output).is_err() {
                output.fill([0.0; 2]);
                failed.store(true, Ordering::Release);
            }
        })?
    };
    if failed.load(Ordering::Acquire) {
        return Err("music rendering failed during startup".into());
    }
    ready.send(Ok(()))?;
    loop {
        match stop.recv_timeout(Duration::from_millis(100)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if queue.status() != 0 || failed.load(Ordering::Acquire) {
            failed.store(true, Ordering::Release);
            break;
        }
    }
    // Queue is disposed before music, song and data leave their scopes.
    drop(queue);
    Ok(())
}
