//! Original IntroSoundHandler PCM conversion and 145-second music restart.
use crate::v2m::{
    Error, Song,
    playback::{Backend, Playback},
};
pub const LOOP_FRAMES: i32 = 145 * 44100;
pub struct Music<'s, 'a, B: Backend> {
    pub player: Playback<'s, 'a, B>,
    pub timer: i32,
}
impl<'s, 'a, B: Backend> Music<'s, 'a, B> {
    pub fn new(song: &'s Song<'a>, backend: B) -> Result<Self, Error> {
        let mut player = Playback::new(song, 44100, 1000, backend)?;
        player.play(0)?;
        Ok(Self { player, timer: 0 })
    }
    pub fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error> {
        self.timer = self.timer.wrapping_add(output.len() as i32);
        for chunk in output.chunks_mut(4096) {
            self.player.render(chunk, chunk.len())?;
            for frame in chunk {
                for sample in frame {
                    // Preserve the original signed PCM16 truncation and symmetric
                    // clamp before converting back to the native float device format.
                    *sample = ((*sample * 32767.0) as i32).clamp(-32767, 32767) as f32 / 32768.0;
                }
            }
        }
        if self.timer >= LOOP_FRAMES {
            // Original Open(song) resets the synth, and Play(0) resets it again.
            self.player.backend.reset()?;
            self.player.play(0)?;
            self.timer = 0;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub mod output;
