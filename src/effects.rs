//! Original PlayerSound/monster sound requests and per-frame 3D commit.
use crate::{
    game::{player::Sound, simulation::Tick},
    sample_playback::{Listener, Pool, Spatial},
    vfx::Sample,
};
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
    pub fn tick(&mut self, tick: &Tick, now: i32) {
        for audio in &tick.sounds {
            self.play(audio.sound, audio.position);
        }
        for listener in &tick.listeners {
            self.pool.set_listener(Listener {
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
            });
        }
        // mainplayer commits once after OnTick, including frames without a tick.
        self.pool.commit(now);
    }
}
