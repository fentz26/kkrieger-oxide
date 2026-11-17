//! Original oscillator workspace, pitch conversion and waveform parameters.
use super::{math::integer, rate::Rate};
#[derive(Clone, Copy)]
pub struct Parameters {
    pub mode: f32,
    pub ring: f32,
    pub pitch: f32,
    pub detune: f32,
    pub color: f32,
    pub gain: f32,
}
#[derive(Clone, Copy, Default)]
pub struct Oscillator {
    pub mode: u32,
    pub ring: u32,
    pub counter: u32,
    pub frequency: u32,
    pub breakpoint: u32,
    pub noise_frequency: f32,
    pub noise_resonance: f32,
    pub seed: u32,
    pub gain: f32,
    pub oversampled_gain: f32,
    pub temporary: u32,
    pub noise_low: f32,
    pub noise_band: f32,
    pub note: f32,
    pub pitch: f32,
}
impl Oscillator {
    /// The native initializer obtains this seed with RDTSC.
    pub fn init(&mut self, seed: u32) {
        self.counter = 0;
        self.noise_low = 0.;
        self.noise_band = 0.;
        self.seed = seed;
    }
    pub fn set(&mut self, p: Parameters, rate: &Rate) {
        self.mode = integer(p.mode) as u32;
        self.ring = integer(p.ring) as u32;
        let detune = (p.detune - 64.) * (1. / 128.);
        self.pitch = (p.pitch - 64.) + detune;
        self.change_pitch(rate);
        self.gain = p.gain * (1. / 128.);
        self.oversampled_gain = self.gain * 0.25;
        let color = p.color * (1. / 128.);
        self.breakpoint = (integer(color * 2147483648.) as u32).wrapping_shl(1);
        self.noise_resonance = 1. - f64::from(color).sqrt() as f32;
    }
    pub fn change_pitch(&mut self, rate: &Rate) {
        let color = (self.pitch + 64.) * (1. / 128.);
        let exponent = (color - 1.) * 10.;
        self.noise_frequency = (f64::from(exponent).exp2() as f32) * rate.linear_frequency;
        let exponent = ((self.pitch + self.note) - 60.) * f32::from_bits(0x3daaaaab);
        self.frequency = integer((f64::from(exponent).exp2() as f32) * rate.oscillator_base) as u32;
    }
}
