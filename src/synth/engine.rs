//! Original MIDI-to-audio orchestration, fixed control ticks and frame caching.
use super::{
    allocation::{Allocator, VOICES},
    channel::{self, Channel},
    controls::{self, Changes},
    master::Master,
    midi::{Decoder, Message},
    patch::Bank,
    rate::Rate,
    voice::{self, MAX_FRAME_SIZE, Voice},
};
/// Speech synthesis follows the original Ronan callback boundaries. The engine
/// requires a speech implementation rather than silently discarding channel 16.
pub trait Speech {
    fn reset(&mut self, sample_rate: u32) -> Result<(), &'static str>;
    fn note_on(&mut self) -> Result<(), &'static str>;
    fn note_off(&mut self) -> Result<(), &'static str>;
    fn control(&mut self, index: u8, value: u8) -> Result<(), &'static str>;
    fn tick(&mut self) -> Result<(), &'static str>;
    fn render(&mut self, channel: &mut [[f32; 2]]) -> Result<(), &'static str>;
}
struct Buffers {
    mix: [[f32; 2]; MAX_FRAME_SIZE],
    channel: [[f32; 2]; MAX_FRAME_SIZE],
    auxiliary1: [f32; MAX_FRAME_SIZE],
    auxiliary2: [f32; MAX_FRAME_SIZE],
    auxiliary_a: [[f32; 2]; MAX_FRAME_SIZE],
    auxiliary_b: [[f32; 2]; MAX_FRAME_SIZE],
    levels: [[f32; 2]; MAX_FRAME_SIZE],
    voice: voice::Buffers,
}
impl Default for Buffers {
    fn default() -> Self {
        Self {
            mix: [[0.; 2]; MAX_FRAME_SIZE],
            channel: [[0.; 2]; MAX_FRAME_SIZE],
            auxiliary1: [0.; MAX_FRAME_SIZE],
            auxiliary2: [0.; MAX_FRAME_SIZE],
            auxiliary_a: [[0.; 2]; MAX_FRAME_SIZE],
            auxiliary_b: [[0.; 2]; MAX_FRAME_SIZE],
            levels: [[0.; 2]; MAX_FRAME_SIZE],
            voice: voice::Buffers::default(),
        }
    }
}
impl Buffers {
    fn clear(&mut self, n: usize) {
        self.mix[..n].fill([0.; 2]);
        self.auxiliary1[..n].fill(0.);
        self.auxiliary2[..n].fill(0.);
        // Native RenderBlock saves the stereo count in EAX before REP STOSD.
        let value = f32::from_bits((n * 2) as u32);
        self.auxiliary_a[..n].fill([value; 2]);
        self.auxiliary_b[..n].fill([value; 2]);
    }
}
struct Core<'a> {
    bank: Bank<'a>,
    rate: Rate,
    allocator: Allocator,
    controls: controls::State,
    voices: [Voice; VOICES],
    channels: [Channel; 16],
    master: Master,
    buffers: Buffers,
    decoder: Decoder,
    remaining: usize,
    seed: u32,
}
fn seeds(seed: &mut u32) -> [u32; 5] {
    std::array::from_fn(|_| {
        *seed = seed.wrapping_mul(196314165).wrapping_add(907633515);
        *seed
    })
}
impl<'a> Core<'a> {
    fn new(bank: Bank<'a>, sample_rate: u32, mut seed: u32) -> Result<Self, &'static str> {
        let rate = Rate::new(sample_rate)?;
        if rate.frame_size == 0 || rate.frame_size as usize > MAX_FRAME_SIZE {
            return Err("V2 sample rate exceeds fixed frame storage");
        }
        let voices = std::array::from_fn(|_| {
            let mut v = Voice::default();
            v.init(seeds(&mut seed));
            v
        });
        let channels = std::array::from_fn(|_| {
            let mut c = Channel::default();
            c.init();
            c
        });
        let mut master = Master::default();
        master.init();
        Ok(Self {
            bank,
            rate,
            allocator: Allocator::default(),
            controls: controls::State::new(sample_rate),
            voices,
            channels,
            master,
            buffers: Buffers::default(),
            decoder: Decoder::default(),
            remaining: 0,
            seed,
        })
    }
    fn set_voice(&mut self, index: usize) -> Result<(), &'static str> {
        let channel = self.allocator.channels[index] as usize;
        let info = self
            .controls
            .channels
            .get(channel)
            .ok_or("invalid V2 voice channel")?;
        let patch = self.bank.patch(info.program)?;
        let values = patch.voice_values(&self.voices[index], &info.controls);
        self.voices[index].set(voice::Parameters::from_values(&values)?, &self.rate)
    }
    fn tick(&mut self) -> Result<(), &'static str> {
        for index in 0..VOICES {
            if self.allocator.channels[index] < 0 {
                continue;
            }
            self.set_voice(index)?;
            self.voices[index].tick()?;
            if self.voices[index].envelopes[0].state == 0 {
                self.allocator.channels[index] = -1;
            }
        }
        for channel in 0..16 {
            let info = self.controls.channels[channel];
            let latest = self.allocator.latest[channel] as usize;
            let voice = self.voices.get(latest).ok_or("invalid V2 latest voice")?;
            let values = self
                .bank
                .patch(info.program)?
                .channel_values(voice, &info.controls);
            self.channels[channel].set(channel::Parameters::from_values(&values)?, &self.rate)?;
        }
        Ok(())
    }
    fn render_block(&mut self, speech: &mut impl Speech) -> Result<(), &'static str> {
        let n = self.rate.frame_size as usize;
        self.buffers.clear(n);
        for channel in 0..16 {
            if !self.allocator.channels.contains(&(channel as i32)) {
                continue;
            }
            let b = &mut self.buffers;
            b.channel[..n].fill([0.; 2]);
            for index in 0..VOICES {
                if self.allocator.channels[index] != channel as i32 {
                    continue;
                }
                self.voices[index].render(
                    &mut b.channel[..n],
                    &b.auxiliary_a[..n],
                    &b.auxiliary_b[..n],
                    &mut b.voice,
                )?;
            }
            if channel == 15 {
                speech.render(&mut b.channel[..n])?;
            }
            self.channels[channel].process(
                &mut b.channel[..n],
                &mut channel::Buses {
                    mix: &mut b.mix[..n],
                    auxiliary1: &mut b.auxiliary1[..n],
                    auxiliary2: &mut b.auxiliary2[..n],
                    auxiliary_a: &mut b.auxiliary_a[..n],
                    auxiliary_b: &mut b.auxiliary_b[..n],
                },
                &mut b.levels[..n],
            )?;
        }
        let b = &mut self.buffers;
        self.master.process(
            &mut b.mix[..n],
            &b.auxiliary1[..n],
            &b.auxiliary2[..n],
            &mut b.levels[..n],
        )?;
        self.remaining = n;
        Ok(())
    }
}
pub struct Engine<'a, S: Speech> {
    core: Core<'a>,
    pub speech: S,
}
impl<'a, S: Speech> Engine<'a, S> {
    /// The explicit seed makes the original RDTSC-seeded oscillators/LFOs
    /// reproducible without changing their noise or modulation algorithms.
    pub fn new(
        bank: &'a [u8],
        sample_rate: u32,
        seed: u32,
        mut speech: S,
    ) -> Result<Self, &'static str> {
        let core = Core::new(Bank::new(bank)?, sample_rate, seed)?;
        speech.reset(sample_rate)?;
        Ok(Self { core, speech })
    }
    pub fn set_globals(&mut self, globals: &[u8]) -> Result<(), &'static str> {
        self.core.master.set(globals, &self.core.rate)
    }
    pub fn allocator(&self) -> &Allocator {
        &self.core.allocator
    }
    pub fn controls(&self) -> &controls::State {
        &self.core.controls
    }
    pub fn voices(&self) -> &[Voice; VOICES] {
        &self.core.voices
    }
    pub fn remaining(&self) -> usize {
        self.core.remaining
    }
    fn changes(&mut self, changes: Changes) -> Result<(), &'static str> {
        match changes.speech {
            Some(controls::Speech::Release) => self.speech.note_off()?,
            Some(controls::Speech::Controller { index, value }) => {
                self.speech.control(index, value)?
            }
            None => {}
        }
        for index in 0..VOICES {
            if changes.reset & (1 << index) != 0 {
                self.core.voices[index].init(seeds(&mut self.core.seed));
            }
            if changes.release & (1 << index) != 0 {
                self.core.voices[index].note_off();
            }
        }
        Ok(())
    }
    fn message(&mut self, message: Message) -> Result<(), &'static str> {
        let channel = message.status & 15;
        match message.status >> 4 {
            8 | 9 if message.status >> 4 == 8 || message.data[1] == 0 => {
                let notes = std::array::from_fn(|i| self.core.voices[i].note);
                let gates = std::array::from_fn(|i| self.core.voices[i].gate);
                let changes = self.core.controls.note_off(
                    channel,
                    message.data[0],
                    &self.core.allocator,
                    &notes,
                    &gates,
                )?;
                self.changes(changes)?;
            }
            9 => {
                if channel == 15 {
                    self.speech.note_on()?;
                }
                let info = self.core.controls.channels[usize::from(channel)];
                let limit = self.core.bank.patch(info.program)?.maximum_polyphony;
                let gates = std::array::from_fn(|i| self.core.voices[i].gate);
                let index = self.core.allocator.allocate(channel, limit, &gates)?;
                // The original converts patch values using the previous note and
                // velocity before triggering the new note.
                self.core.set_voice(index)?;
                self.core.voices[index].note_on(
                    i32::from(message.data[0]),
                    i32::from(message.data[1]),
                    &self.core.rate,
                );
            }
            11 => {
                let changes = self.core.controls.control(
                    channel,
                    message.data[0],
                    message.data[1],
                    &mut self.core.allocator,
                )?;
                self.changes(changes)?;
            }
            12 => self
                .core
                .controls
                .program(channel, message.data[0], &mut self.core.allocator)?,
            15 if channel == 15 => {
                self.core = Core::new(
                    self.core.bank,
                    self.core.controls.sample_rate,
                    self.core.seed,
                )?;
                self.speech.reset(self.core.controls.sample_rate)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn midi(&mut self, packet: &[u8]) -> Result<usize, &'static str> {
        let mut decoder = self.core.decoder;
        let result = decoder.process(packet, |message| self.message(message));
        self.core.decoder = decoder;
        result
    }
    pub fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), &'static str> {
        let mut done = 0;
        while done < output.len() {
            if self.core.remaining == 0 {
                self.core.tick()?;
                self.speech.tick()?;
                self.core.render_block(&mut self.speech)?;
            }
            let start = self.core.rate.frame_size as usize - self.core.remaining;
            let count = self.core.remaining.min(output.len() - done);
            output[done..done + count]
                .copy_from_slice(&self.core.buffers.mix[start..start + count]);
            self.core.remaining -= count;
            done += count;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_frame_clear_preserves_tail_and_channel_and_fills_auxiliary_bits() {
        let mut bytes = &include_bytes!("../../tests/fixtures/synth-frame-clear.bin")[..];
        fn word(bytes: &mut &[u8]) -> u32 {
            let value = u32::from_le_bytes(bytes[..4].try_into().unwrap());
            *bytes = &bytes[4..];
            value
        }
        assert_eq!(word(&mut bytes), 6);
        for case in 0..6 {
            let count = word(&mut bytes) as usize;
            let mut b = Buffers::default();
            b.mix.fill([0.5; 2]);
            b.channel.fill([0.5; 2]);
            b.auxiliary1.fill(0.5);
            b.auxiliary2.fill(0.5);
            b.auxiliary_a.fill([0.5; 2]);
            b.auxiliary_b.fill([0.5; 2]);
            b.clear(count);
            for (index, &value) in b
                .mix
                .as_flattened()
                .iter()
                .chain(b.channel.as_flattened())
                .chain(&b.auxiliary1)
                .chain(&b.auxiliary2)
                .chain(b.auxiliary_a.as_flattened())
                .chain(b.auxiliary_b.as_flattened())
                .enumerate()
            {
                assert_eq!(
                    value.to_bits(),
                    word(&mut bytes),
                    "case {case} word {index}"
                );
            }
        }
        assert!(bytes.is_empty());
    }
}
