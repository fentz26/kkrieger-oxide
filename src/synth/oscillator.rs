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

impl Oscillator {
    pub fn render(
        &mut self,
        destination: &mut [f32],
        auxiliary_a: &[[f32; 2]],
        auxiliary_b: &[[f32; 2]],
    ) -> Result<(), &'static str> {
        let mode = self.mode & 7;
        if destination.is_empty() || mode == 0 {
            return Ok(());
        }
        if mode >= 6 {
            let source = if mode == 6 { auxiliary_a } else { auxiliary_b };
            if source.len() < destination.len() {
                return Err("oscillator auxiliary buffer too short");
            }
            for (output, input) in destination.iter_mut().zip(source) {
                let value = ((input[0] + input[1]) * self.gain) * 0.6;
                *output = if self.ring & 1 != 0 {
                    value * *output
                } else {
                    value
                };
            }
            return Ok(());
        }
        let (f, breakpoint) = if mode <= 2 {
            (
                fraction(self.frequency),
                f32::from_bits((self.breakpoint >> 9) | 0x3f800000),
            )
        } else {
            (0., 0.)
        };
        let (reciprocal, c1, c2) = if mode == 1 {
            let color = breakpoint - 1.;
            (1. / f, self.gain / color, -(self.gain / (1. - color)))
        } else {
            (0., 0., 0.)
        };
        let (gain_over_frequency, pulse_212, pulse_121) = if mode == 2 {
            let gain_over_frequency = self.gain / f;
            let twice_breakpoint = breakpoint + breakpoint;
            (
                gain_over_frequency,
                ((twice_breakpoint - 2.) * gain_over_frequency) - self.gain,
                ((twice_breakpoint - 4.) * gain_over_frequency) + self.gain,
            )
        } else {
            (0., 0., 0.)
        };
        let mut shape_state = if mode <= 2 {
            let previous = self.counter.wrapping_sub(self.frequency);
            (u32::from(previous < self.breakpoint) << 1) | u32::from(self.counter < self.frequency)
        } else {
            0
        };
        for output in destination {
            let old = *output;
            let value = match mode {
                1 | 2 => {
                    self.temporary = (self.counter >> 9) | 0x3f800000;
                    let phase = f32::from_bits(self.temporary);
                    let transition =
                        ((shape_state << 1) | u32::from(self.counter < self.breakpoint)) & 7;
                    let value = if mode == 1 {
                        let p = phase - breakpoint;
                        let y = match transition {
                            0 | 6 => ((p + p) - f) * c2,
                            1 | 5 => ((p + p) - f) * c1,
                            3 => {
                                let q = (p + 1.) - f;
                                let left = (q * q) * c2;
                                let right = (p * p) * c1;
                                -((left - right) + self.gain) * reciprocal
                            }
                            2 | 7 => {
                                let twice = p + p;
                                let remaining = 1. - f;
                                let slope = if transition == 7 { c1 } else { c2 };
                                -(((remaining * (twice + remaining)) * slope) + self.gain)
                                    * reciprocal
                            }
                            4 => {
                                let left = (p * p) * c2;
                                let q = p - f;
                                let right = (q * q) * c1;
                                (left - right) * reciprocal
                            }
                            _ => unreachable!(),
                        };
                        y + self.gain
                    } else {
                        match transition {
                            0 | 6 => -self.gain,
                            1 | 5 => self.gain,
                            3 => (((phase - 1.) * 2.) * gain_over_frequency) - self.gain,
                            4 => (((breakpoint - phase) * 2.) * gain_over_frequency) + self.gain,
                            2 => pulse_212,
                            7 => pulse_121,
                            _ => unreachable!(),
                        }
                    };
                    let (counter, carry) = self.counter.overflowing_add(self.frequency);
                    self.counter = counter;
                    shape_state = (transition << 1) | u32::from(carry);
                    value
                }
                3 => {
                    let phase = self.counter.wrapping_add(0x40000000);
                    let folded = phase ^ ((phase as i32 >> 31) as u32);
                    self.temporary = (folded >> 8) | 0x3f800000;
                    let angle = f32::from_bits(self.temporary) * std::f32::consts::PI
                        - std::f32::consts::PI * 1.5;
                    self.counter = self.counter.wrapping_add(self.frequency);
                    super::math::sine_polynomial(angle) * self.gain
                }
                4 => {
                    self.seed = self.seed.wrapping_mul(196314165).wrapping_add(907633515);
                    self.temporary = (self.seed >> 9) | 0x40000000;
                    let noise = ((f32::from_bits(self.temporary) - 1.) - 1.) - 1.;
                    let damped = self.noise_band * self.noise_resonance;
                    self.noise_low += self.noise_band * self.noise_frequency;
                    let high = (noise - damped) - self.noise_low;
                    self.noise_band += high * self.noise_frequency;
                    (((high + self.noise_low) * self.noise_resonance) + self.noise_band) * self.gain
                }
                5 => {
                    self.temporary = (self.counter >> 9) | 0x3f800000;
                    self.counter = self.counter.wrapping_add(self.frequency);
                    let phase = (old * 2.) + f32::from_bits(self.temporary);
                    super::math::fast_sine(phase * std::f32::consts::TAU) * self.gain
                }
                _ => unreachable!(),
            };
            *output = if self.ring & 1 != 0 {
                value * old
            } else if mode == 5 {
                value
            } else {
                value + old
            };
        }
        Ok(())
    }
}
fn fraction(counter: u32) -> f32 {
    f32::from_bits((counter >> 9) | 0x3f800000) - 1.
}
