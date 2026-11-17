//! Original ViruZ II per-voice setup, gate events, control tick and stereo mix.
use super::{distortion, envelope, filter, lfo, math::integer, oscillator, rate::Rate};
pub const MAX_FRAME_SIZE: usize = 280;
#[derive(Clone, Copy)]
pub struct Parameters {
    pub panning: f32,
    pub transpose: f32,
    pub oscillators: [oscillator::Parameters; 3],
    pub filters: [filter::Parameters; 2],
    pub routing: f32,
    pub filter_balance: f32,
    pub distortion: distortion::Parameters,
    pub envelopes: [envelope::Parameters; 2],
    pub lfos: [lfo::Parameters; 2],
    pub oscillator_sync: f32,
}
impl Parameters {
    pub fn from_values(values: &[f32]) -> Result<Self, &'static str> {
        let p = values.get(..59).ok_or("truncated V2 voice parameters")?;
        Ok(Self {
            panning: p[0],
            transpose: p[1],
            oscillators: std::array::from_fn(|i| {
                let n = 2 + i * 6;
                oscillator::Parameters {
                    mode: p[n],
                    ring: p[n + 1],
                    pitch: p[n + 2],
                    detune: p[n + 3],
                    color: p[n + 4],
                    gain: p[n + 5],
                }
            }),
            filters: std::array::from_fn(|i| {
                let n = 20 + i * 3;
                filter::Parameters {
                    mode: p[n],
                    cutoff: p[n + 1],
                    resonance: p[n + 2],
                }
            }),
            routing: p[26],
            filter_balance: p[27],
            distortion: distortion::Parameters {
                mode: p[28],
                input_gain: p[29],
                parameter1: p[30],
                parameter2: p[31],
            },
            envelopes: std::array::from_fn(|i| {
                let n = 32 + i * 6;
                envelope::Parameters {
                    attack: p[n],
                    decay: p[n + 1],
                    sustain_level: p[n + 2],
                    sustain_rate: p[n + 3],
                    release: p[n + 4],
                    volume: p[n + 5],
                }
            }),
            lfos: std::array::from_fn(|i| {
                let n = 44 + i * 7;
                lfo::Parameters {
                    mode: p[n],
                    sync: p[n + 1],
                    envelope: p[n + 2],
                    rate: p[n + 3],
                    phase: p[n + 4],
                    polarity: p[n + 5],
                    amplitude: p[n + 6],
                }
            }),
            oscillator_sync: p[58],
        })
    }
}
#[derive(Clone, Copy, Default)]
pub struct Voice {
    pub note: i32,
    pub velocity: f32,
    pub gate: u32,
    pub current_volume: f32,
    pub volume_ramp: f32,
    pub transpose: f32,
    pub routing: u32,
    pub left_volume: f32,
    pub right_volume: f32,
    pub filter1_gain: f32,
    pub filter2_gain: f32,
    pub oscillator_sync: u32,
    pub oscillators: [oscillator::Oscillator; 3],
    pub filters: [filter::Filter; 2],
    pub envelopes: [envelope::Envelope; 2],
    pub lfos: [lfo::Lfo; 2],
    pub distortion: distortion::Distortion,
}
pub struct Buffers {
    mono: [f32; MAX_FRAME_SIZE],
    parallel: [f32; MAX_FRAME_SIZE],
}
impl Default for Buffers {
    fn default() -> Self {
        Self {
            mono: [0.; MAX_FRAME_SIZE],
            parallel: [0.; MAX_FRAME_SIZE],
        }
    }
}
impl Voice {
    pub fn init(&mut self, seeds: [u32; 5]) {
        for (oscillator, seed) in self.oscillators.iter_mut().zip(seeds) {
            oscillator.init(seed);
        }
        for envelope in &mut self.envelopes {
            envelope.init();
        }
        for filter in &mut self.filters {
            filter.init();
        }
        for (lfo, seed) in self.lfos.iter_mut().zip(&seeds[3..]) {
            lfo.init(*seed);
        }
        self.distortion.init();
    }
    pub fn set(&mut self, p: Parameters, rate: &Rate) -> Result<(), &'static str> {
        self.transpose = p.transpose - 64.;
        let note = self.transpose + self.note as f32;
        for oscillator in &mut self.oscillators {
            oscillator.note = note;
        }
        self.routing = integer(p.routing) as u32;
        self.oscillator_sync = integer(p.oscillator_sync) as u32;
        let pan = p.panning * (1. / 128.);
        self.left_volume = f64::from(1. - pan).sqrt() as f32;
        self.right_volume = f64::from(pan).sqrt() as f32;
        let balance = p.filter_balance - 64.;
        let negative = integer(balance) < 0;
        let balance = balance * (1. / 64.);
        (self.filter1_gain, self.filter2_gain) = if negative {
            (1., balance + 1.)
        } else {
            (1. - balance, 1.)
        };
        for (oscillator, p) in self.oscillators.iter_mut().zip(p.oscillators) {
            oscillator.set(p, rate);
        }
        for (envelope, p) in self.envelopes.iter_mut().zip(p.envelopes) {
            envelope.set(p);
        }
        for (filter, p) in self.filters.iter_mut().zip(p.filters) {
            filter.set(p, rate.linear_frequency);
        }
        for (lfo, p) in self.lfos.iter_mut().zip(p.lfos) {
            lfo.set(p);
        }
        self.distortion.set(p.distortion, rate.linear_frequency)
    }
    pub fn note_on(&mut self, note: i32, velocity: i32, rate: &Rate) {
        self.note = note;
        self.velocity = velocity as f32;
        self.gate = 1;
        for envelope in &mut self.envelopes {
            envelope.state = 1;
        }
        for oscillator in &mut self.oscillators {
            oscillator.note = note as f32 + self.transpose;
            if self.oscillator_sync != 0 {
                oscillator.counter = 0;
            }
            oscillator.change_pitch(rate);
        }
        for lfo in &mut self.lfos {
            lfo.key_on();
        }
    }
    pub fn note_off(&mut self) {
        self.gate = 0;
    }
    pub fn tick(&mut self) -> Result<(), &'static str> {
        for envelope in &mut self.envelopes {
            envelope.tick(self.gate != 0)?;
        }
        for lfo in &mut self.lfos {
            lfo.tick()?;
        }
        // The published synth uses a fixed 1/128 ramp, even at other rates.
        self.volume_ramp =
            (self.envelopes[0].output * (1. / 128.) - self.current_volume) * (1. / 128.);
        Ok(())
    }
    // Preserve the assembly operand order, including non-finite sample behavior.
    #[allow(clippy::assign_op_pattern)]
    pub fn render(
        &mut self,
        channel: &mut [[f32; 2]],
        auxiliary_a: &[[f32; 2]],
        auxiliary_b: &[[f32; 2]],
        buffers: &mut Buffers,
    ) -> Result<(), &'static str> {
        let count = channel.len();
        if count > MAX_FRAME_SIZE {
            return Err("V2 voice frame exceeds 280 samples");
        }
        if count == 0 {
            return Ok(());
        }
        for oscillator in &self.oscillators {
            match oscillator.mode & 7 {
                6 if auxiliary_a.len() < count => return Err("voice auxiliary A buffer too short"),
                7 if auxiliary_b.len() < count => return Err("voice auxiliary B buffer too short"),
                _ => {}
            }
        }
        buffers.mono[..count].fill(0.);
        for oscillator in &mut self.oscillators {
            oscillator.render(&mut buffers.mono[..count], auxiliary_a, auxiliary_b)?;
        }
        let routing = self.routing as u8;
        if routing == 2 {
            self.filters[1].render(
                &mut buffers.parallel[..count],
                &buffers.mono[..count],
                count,
            )?;
        }
        self.filters[0].render_in_place(&mut buffers.mono[..count], count)?;
        if routing == 2 {
            for i in 0..count {
                buffers.mono[i] =
                    buffers.parallel[i] * self.filter2_gain + buffers.mono[i] * self.filter1_gain;
            }
        } else if routing != 0 {
            self.filters[1].render_in_place(&mut buffers.mono[..count], count)?;
        }
        self.distortion
            .render_in_place(&mut buffers.mono[..count], false)?;
        for (output, &input) in channel.iter_mut().zip(&buffers.mono[..count]) {
            let value = input * self.current_volume;
            self.current_volume += self.volume_ramp;
            let left = value * self.left_volume;
            let right = value * self.right_volume;
            output[0] = (left + f32::from_bits(0x36800000)) + output[0];
            output[1] = (right + f32::from_bits(0x36800000)) + output[1];
        }
        Ok(())
    }
}
