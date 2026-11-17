//! Original CV2MPlayer event/sample scheduling, seeks and bounded legacy fades.
use super::{
    Error, Song,
    sequence::{Sequencer, sample_delta},
};
pub trait Backend {
    fn reset(&mut self) -> Result<(), Error>;
    fn midi(&mut self, packet: &[u8]) -> Result<(), Error>;
    fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum State {
    Off,
    Stopped,
    Playing,
}
pub struct Playback<'s, 'a, B: Backend> {
    pub backend: B,
    pub sequence: Sequencer<'s, 'a>,
    pub state: State,
    pub valid: bool,
    pub current_sample: u32,
    pub sample_delta: u32,
    pub remainder: u32,
    pub time_offset: i32,
    pub fade_value: f32,
    pub fade_delta: f32,
    song: &'s Song<'a>,
    sample_rate: u32,
    ticks_per_second: u32,
}
/// Original signed x86 IMUL/IDIV conversion, with division faults reported.
pub fn samples_for_time(time: u32, sample_rate: u32, ticks_per_second: u32) -> Result<u32, Error> {
    let divisor = ticks_per_second as i32;
    if divisor == 0 {
        return Err(Error("zero V2M timebase"));
    }
    let product = i64::from(time as i32) * i64::from(sample_rate as i32);
    let result = product / i64::from(divisor);
    Ok(i32::try_from(result).map_err(|_| Error("V2M time conversion overflow"))? as u32)
}
impl<'s, 'a, B: Backend> Playback<'s, 'a, B> {
    pub fn new(
        song: &'s Song<'a>,
        sample_rate: u32,
        ticks_per_second: u32,
        backend: B,
    ) -> Result<Self, Error> {
        if ticks_per_second == 0 {
            return Err(Error("zero V2M timebase"));
        }
        let mut sequence = Sequencer::new(song, sample_rate)?;
        sequence.playing = false;
        let mut player = Self {
            backend,
            sequence,
            state: State::Off,
            valid: true,
            current_sample: 0,
            sample_delta: 0,
            remainder: 0,
            time_offset: 0,
            fade_value: 0.,
            fade_delta: 0.,
            song,
            sample_rate,
            ticks_per_second,
        };
        if sample_rate != 0 {
            player.backend.reset()?;
        }
        Ok(player)
    }
    fn tick(&mut self) -> Result<(), Error> {
        if self.state != State::Playing {
            return Ok(());
        }
        self.sequence.advance()?;
        self.backend.midi(&self.sequence.midi)?;
        if !self.sequence.playing {
            self.state = State::Stopped;
        }
        Ok(())
    }
    fn next_delta(&mut self) -> Result<(), Error> {
        self.sample_delta = if self.state == State::Playing {
            sample_delta(
                self.sequence.next_time,
                self.sequence.time,
                self.sequence.microseconds,
                self.song.scaled_time_division,
                &mut self.remainder,
            )?
        } else {
            u32::MAX
        };
        Ok(())
    }
    pub fn play(&mut self, time: u32) -> Result<(), Error> {
        if !self.valid || self.sample_rate == 0 {
            return Ok(());
        }
        self.stop(0)?;
        self.sequence = Sequencer::new(self.song, self.sample_rate)?;
        self.remainder = 0;
        self.backend.reset()?;
        let destination = samples_for_time(time, self.sample_rate, self.ticks_per_second)?;
        self.state = State::Playing;
        self.sample_delta = 0;
        self.remainder = 0;
        let mut cursor = 0u32;
        // Seeking dispatches MIDI without rendering elapsed audio in the original.
        while cursor.wrapping_add(self.sample_delta) < destination && self.state == State::Playing {
            cursor = cursor.wrapping_add(self.sample_delta);
            self.tick()?;
            self.next_delta()?;
        }
        self.sample_delta = self
            .sample_delta
            .wrapping_sub(destination.wrapping_sub(cursor));
        self.time_offset = cursor.wrapping_sub(self.current_sample) as i32;
        self.fade_value = 1.;
        self.fade_delta = 0.;
        Ok(())
    }
    pub fn stop(&mut self, fade_time: u32) -> Result<(), Error> {
        if !self.valid {
            return Ok(());
        }
        if fade_time != 0 {
            let samples = samples_for_time(fade_time, self.sample_rate, self.ticks_per_second)?;
            self.fade_delta = self.fade_value / samples as f32;
        } else {
            self.state = State::Off;
            self.sequence.playing = false;
        }
        Ok(())
    }
    pub fn close(&mut self) -> Result<(), Error> {
        if !self.valid {
            return Ok(());
        }
        if self.state != State::Off {
            self.stop(0)?;
        }
        self.valid = false;
        Ok(())
    }
    /// Render `frames` stereo samples. While PLAYING, the published fade loop
    /// addresses the following `frames` samples; supply that tail when fading.
    /// The full touched range is validated before any playback or audio changes.
    pub fn render(&mut self, output: &mut [[f32; 2]], frames: usize) -> Result<bool, Error> {
        if frames > i32::MAX as usize {
            return Err(Error("V2M render count exceeds signed sample range"));
        }
        let playing = self.valid && self.state == State::Playing;
        let fade_offset = if playing { frames } else { 0 };
        let required = if self.fade_delta != 0. {
            fade_offset
                .checked_add(frames)
                .ok_or(Error("V2M fade range overflow"))?
        } else {
            frames
        };
        if output.len() < required {
            return Err(Error("V2M output buffer lacks render or legacy fade tail"));
        }
        if playing {
            let mut done = 0;
            while done < frames {
                let count = (frames - done).min(self.sample_delta as usize);
                if count != 0 {
                    self.backend.render(&mut output[done..done + count])?;
                    done += count;
                    self.sample_delta = self.sample_delta.wrapping_sub(count as u32);
                    self.current_sample = self.current_sample.wrapping_add(count as u32);
                }
                if self.sample_delta == 0 {
                    self.tick()?;
                    self.next_delta()?;
                }
            }
        } else if self.state == State::Off || !self.valid {
            output[..frames].fill([0.; 2]);
        } else {
            self.backend.render(&mut output[..frames])?;
            self.current_sample = self.current_sample.wrapping_add(frames as u32);
        }
        if self.fade_delta != 0. {
            for frame in &mut output[fade_offset..fade_offset + frames] {
                frame[0] *= self.fade_value;
                frame[1] *= self.fade_value;
                self.fade_value -= self.fade_delta;
                if self.fade_value < 0. {
                    self.fade_value = 0.;
                }
            }
            if self.fade_value == 0. {
                self.stop(0)?;
            }
        }
        Ok(self.valid && self.state == State::Playing)
    }
}

/// Adapter retaining the song's global parameters and lyric references across
/// the original synth reinitializations performed by Open and Play.
pub struct Synth<'a> {
    pub engine: crate::synth::engine::Engine<'a, crate::synth::speech::Ronan<'a>>,
    globals: &'a [u8],
    lyrics: [Option<&'a [u8]>; 64],
    sample_rate: u32,
    first_reset: bool,
}
impl<'a> Synth<'a> {
    pub fn new(song: &Song<'a>, sample_rate: u32, seed: u32) -> Result<Self, Error> {
        let mut engine = crate::synth::engine::Engine::new(
            song.patches,
            sample_rate,
            seed,
            crate::synth::speech::Ronan::default(),
        )
        .map_err(Error)?;
        engine.set_globals(song.globals).map_err(Error)?;
        let lyrics = std::array::from_fn(|i| song.speech[i]);
        engine.speech.set_lyrics(&lyrics).map_err(Error)?;
        Ok(Self {
            engine,
            globals: song.globals,
            lyrics,
            sample_rate,
            first_reset: true,
        })
    }
}
impl Backend for Synth<'_> {
    fn reset(&mut self) -> Result<(), Error> {
        if self.first_reset {
            self.first_reset = false;
        } else {
            self.engine.reset(self.sample_rate).map_err(Error)?;
        }
        self.engine.set_globals(self.globals).map_err(Error)?;
        self.engine.speech.set_lyrics(&self.lyrics).map_err(Error)
    }
    fn midi(&mut self, packet: &[u8]) -> Result<(), Error> {
        self.engine.midi(packet).map(|_| ()).map_err(Error)
    }
    fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error> {
        self.engine.render(output).map_err(Error)
    }
}
