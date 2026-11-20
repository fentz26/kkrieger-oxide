//! Own song storage on a worker while the native queue borrows its synthesizer.
use super::Music;
use crate::{
    audio_output::Queue,
    effects::{Effects, Update, updates},
    game::{player::Sound, simulation::Tick},
    v2m::{Song, conversion::convert, playback::Synth},
    vfx::Bank,
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
const REQUEST_CAPACITY: usize = 1024;
#[derive(Clone, Copy)]
enum Command {
    Effect(Update),
    PauseMusic,
    RestartMusic,
}
pub struct Output {
    requests: SyncSender<Command>,
    stop: SyncSender<()>,
    worker: Option<JoinHandle<()>>,
    failed: Arc<AtomicBool>,
}
impl Output {
    pub fn open(song: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start(song, None)
    }
    pub fn open_with_effects(
        song: &[u8],
        samples: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start(song, Some(samples.to_vec()))
    }
    fn start(song: &[u8], samples: Option<Vec<u8>>) -> Result<Self, Box<dyn std::error::Error>> {
        let data = convert(song)?;
        let (requests, commands) = mpsc::sync_channel(REQUEST_CAPACITY);
        let (stop, receiver) = mpsc::sync_channel(1);
        let (ready, started) = mpsc::sync_channel(1);
        let failed = Arc::new(AtomicBool::new(false));
        let thread_failed = Arc::clone(&failed);
        let worker = thread::Builder::new()
            .name("kkrieger-audio".into())
            .spawn(move || {
                if let Err(error) = run(
                    &data,
                    samples.as_deref(),
                    commands,
                    receiver,
                    &ready,
                    &thread_failed,
                ) {
                    thread_failed.store(true, Ordering::Release);
                    let _ = ready.send(Err(error.to_string()));
                }
            })?;
        match started.recv() {
            Ok(Ok(())) => Ok(Self {
                requests,
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
    pub fn tick(&self, tick: Tick, now: i32) -> Result<(), &'static str> {
        for update in updates(&tick, now) {
            self.requests
                .send(Command::Effect(update))
                .map_err(|_| "audio worker stopped")?;
        }
        // Simulation vectors are dropped here, on the game thread.
        Ok(())
    }
    pub fn play(&self, sound: Sound, position: [f32; 4]) -> Result<(), &'static str> {
        self.requests
            .send(Command::Effect(Update::Play(sound, position)))
            .map_err(|_| "audio worker stopped")
    }
    pub fn pause_music(&self) -> Result<(), &'static str> {
        self.requests
            .send(Command::PauseMusic)
            .map_err(|_| "audio worker stopped")
    }
    pub fn restart_music(&self) -> Result<(), &'static str> {
        self.requests
            .send(Command::RestartMusic)
            .map_err(|_| "audio worker stopped")
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
    sample_data: Option<&[u8]>,
    commands: Receiver<Command>,
    stop: Receiver<()>,
    ready: &SyncSender<Result<(), String>>,
    failed: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error>> {
    let song = Song::parse(data)?;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .subsec_nanos();
    let samples = match sample_data {
        Some(data) => Bank::parse(data)?.render(seed)?,
        None => Vec::new(),
    };
    let mut effects = Effects::new(&samples);
    let synth = Synth::new(&song, 44100, seed)?;
    let mut music = Music::new(&song, synth)?;
    // This queue never escapes this scope and is synchronously disposed on all
    // return/unwind paths before its borrowed music and song storage expires.
    let queue = unsafe {
        Queue::open_scoped(move |output| {
            for command in commands.try_iter().take(REQUEST_CAPACITY) {
                match command {
                    Command::Effect(update) => effects.apply(update),
                    Command::PauseMusic => music.pause(),
                    Command::RestartMusic => {
                        if music.restart().is_err() {
                            failed.store(true, Ordering::Release);
                        }
                    }
                }
            }
            if failed.load(Ordering::Acquire) || music.render(output).is_err() {
                output.fill([0.0; 2]);
                failed.store(true, Ordering::Release);
            } else {
                effects.mix(output);
            }
        })?
    };
    #[cfg(target_os = "windows")]
    let mut queue = queue;
    if failed.load(Ordering::Acquire) {
        return Err("music rendering failed during startup".into());
    }
    ready.send(Ok(()))?;
    loop {
        match stop.recv_timeout(Duration::from_millis(if cfg!(target_os = "windows") {
            5
        } else {
            100
        })) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        #[cfg(target_os = "windows")]
        if queue.pump().is_err() {
            failed.store(true, Ordering::Release);
            break;
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
