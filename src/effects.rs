//! Original PlayerSound/monster sound requests and per-frame 3D commit.
use crate::{
    game::{player::Sound, simulation::Tick},
    sample_playback::{Listener, Pool, Spatial},
    vfx::Sample,
};
/// Fixed-size requests passed from simulation to the audio callback.
#[derive(Clone, Copy)]
pub enum Update {
    Play(Sound, [f32; 4]),
    Listener(Listener),
    Commit(i32),
}
pub fn updates(tick: &Tick, now: i32) -> impl Iterator<Item = Update> + '_ {
    tick.sounds
        .iter()
        .map(|audio| Update::Play(audio.sound, audio.position))
        .chain(tick.listeners.iter().map(|listener| {
            Update::Listener(Listener {
                position: [
                    listener.position[0],
                    listener.position[1],
                    listener.position[2],
                ],
                velocity: [
                    listener.velocity[0],
                    listener.velocity[1],
                    listener.velocity[2],
                ],
                up: [listener.up[0], listener.up[1], listener.up[2]],
                forward: [
                    listener.forward[0],
                    listener.forward[1],
                    listener.forward[2],
                ],
                ..Listener::default()
            })
        }))
        .chain(std::iter::once(Update::Commit(now)))
}
pub struct Effects<'a> {
    pub pool: Pool<'a>,
}
impl<'a> Effects<'a> {
    pub fn new(samples: &'a [Sample]) -> Self {
        let mut pool = Pool::new();
        for sample in samples {
            pool.add(sample);
        }
        Self { pool }
    }
    pub fn play(&mut self, sound: Sound, position: [f32; 4]) -> Option<i32> {
        let slot = u8::try_from(sound.slot).ok()?;
        let token = self.pool.play(slot, sound.volume, 0.0)?;
        self.pool.spatial(
            slot,
            token,
            Spatial {
                position: [position[0], position[1], position[2]],
                // Both PlayerSound and the monster sound helpers pass zero velocity.
                velocity: [0.0; 3],
                near: sound.near_distance,
                far: sound.far_distance,
            },
        );
        Some(token)
    }
    pub fn mix(&mut self, output: &mut [[f32; 2]]) {
        let listener = *self.pool.listener();
        self.pool
            .mix(output, |source| listener.stereo_gains(source));
    }
    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Play(sound, position) => {
                self.play(sound, position);
            }
            Update::Listener(listener) => {
                self.pool.set_listener(listener);
            }
            Update::Commit(now) => self.pool.commit(now),
        }
    }
    pub fn tick(&mut self, tick: &Tick, now: i32) {
        // mainplayer commits once after OnTick, including frames without a tick.
        for update in updates(tick, now) {
            self.apply(update);
        }
    }
}
