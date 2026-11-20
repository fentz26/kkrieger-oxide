//! Original IntroSoundHandler PCM conversion and 145-second music restart.
use crate::v2m::{
    Error, Song,
    playback::{Backend, Playback},
};
pub const LOOP_FRAMES: i32 = 145 * 44100;
pub struct Music<'s, 'a, B: Backend> {
    pub player: Playback<'s, 'a, B>,
    pub timer: i32,
    enabled: bool,
}
impl<'s, 'a, B: Backend> Music<'s, 'a, B> {
    pub fn new(song: &'s Song<'a>, backend: B) -> Result<Self, Error> {
        let mut player = Playback::new(song, 44100, 1000, backend)?;
        player.play(0)?;
        Ok(Self {
            player,
            timer: 0,
            enabled: true,
        })
    }
    /// Removing IntroSoundHandler silences only music and leaves its timer intact.
    pub fn pause(&mut self) {
        self.enabled = false;
    }
    /// Root changes call Open(song), Play(0), then reinstall IntroSoundHandler.
    /// SoundTimer is retained by the original root-change path.
    pub fn restart(&mut self) -> Result<(), Error> {
        self.player.backend.reset()?;
        self.player.play(0)?;
        self.enabled = true;
        Ok(())
    }
    pub fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error> {
        if !self.enabled {
            output.fill([0.0; 2]);
            return Ok(());
        }
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
            self.restart()?;
            self.timer = 0;
        }
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub mod output;
