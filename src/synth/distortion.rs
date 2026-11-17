//! Original ViruZ II mono/stereo distortion and decimator state.
use super::{
    filter::{Filter, Parameters as FilterParameters},
    math::{fast_atan, integer},
};
#[derive(Clone, Copy)]
pub struct Parameters {
    pub mode: f32,
    pub input_gain: f32,
    pub parameter1: f32,
    pub parameter2: f32,
}
#[derive(Clone, Copy, Default)]
pub struct Distortion {
    pub mode: u32,
    pub input_gain: f32,
    pub output_gain: f32,
    pub offset: f32,
    pub crush_scale: f32,
    pub crush_step: i32,
    pub crush_xor: u32,
    pub counter: u32,
    pub frequency: u32,
    pub held_left: f32,
    pub held_right: f32,
    pub pre_coefficient: f32,
    pub pre_left: f32,
    pub pre_right: f32,
    pub post_coefficient: f32,
    pub post_left: f32,
    pub post_right: f32,
    pub left_filter: Filter,
    pub right_filter: Filter,
}
impl Distortion {
    pub fn init(&mut self) {
        self.counter = 0;
        self.held_left = 0.;
        self.held_right = 0.;
        self.pre_left = 0.;
        self.pre_right = 0.;
        self.post_left = 0.;
        self.post_right = 0.;
        self.left_filter.init();
        self.right_filter.init();
    }
    pub fn set(&mut self, p: Parameters, linear_frequency: f32) -> Result<(), &'static str> {
        let mode = integer(p.mode) as u32;
        if mode > 15 {
            return Err("invalid V2 distortion mode");
        }
        self.mode = mode;
        let exponent = (p.input_gain - 32.) * 0.0625;
        self.input_gain = f64::from(exponent).exp2() as f32;
        match mode {
            1 | 2 => {
                let gain = p.parameter1 * (1. / 128.);
                self.output_gain = if mode == 1 {
                    (f64::from(gain) / f64::from(self.input_gain).atan()) as f32
                } else {
                    gain
                };
                self.offset = (((p.parameter2 - 64.) * (1. / 128.)) * 2.) * self.input_gain;
            }
            3 => {
                let step = p.parameter1 * 256. + 1.;
                self.crush_step = integer(step);
                self.crush_scale = (32768. / step) * self.input_gain;
                self.crush_xor = (integer(p.parameter2) as u32).wrapping_shl(9);
            }
            4 => {
                let frequency = f64::from((p.parameter1 * (1. / 128.) - 1.) * 10.).exp2() as f32;
                self.frequency = (integer(frequency * 2147483648.) as u32).wrapping_shl(1);
                let pre = p.input_gain * (1. / 127.);
                self.pre_coefficient = pre * pre;
                let post = p.parameter2 * (1. / 127.);
                self.post_coefficient = post * post;
            }
            5..=9 => {
                let p = FilterParameters {
                    mode: (mode - 4) as f32,
                    cutoff: p.parameter1,
                    resonance: p.parameter2,
                };
                self.left_filter.set(p, linear_frequency);
                self.right_filter.set(p, linear_frequency);
            }
            _ => {}
        }
        Ok(())
    }
    pub fn render(
        &mut self,
        destination: &mut [f32],
        source: &[f32],
        stereo: bool,
    ) -> Result<(), &'static str> {
        self.validate(destination.len(), source.len(), stereo)?;
        if destination.is_empty() {
            return Ok(());
        }
        if (5..=9).contains(&self.mode) {
            if stereo {
                self.left_filter.step_bytes = 8;
                self.right_filter.step_bytes = 8;
                let count = destination.len() / 2;
                self.left_filter.render(destination, source, count)?;
                self.right_filter
                    .render(&mut destination[1..], &source[1..], count)?;
            } else {
                let count = destination.len();
                self.left_filter.render(destination, source, count)?;
            }
        } else if self.mode == 4 {
            let step = if stereo { 2 } else { 1 };
            for i in (0..destination.len()).step_by(step) {
                let (counter, carry) = self.counter.overflowing_add(self.frequency);
                self.counter = counter;
                if carry {
                    self.held_left = source[i];
                    if stereo {
                        self.held_right = source[i + 1];
                    }
                }
                destination[i] = self.held_left;
                if stereo {
                    destination[i + 1] = self.held_right;
                }
            }
        } else {
            for (output, &input) in destination.iter_mut().zip(source) {
                *output = self.scalar(input);
            }
        }
        Ok(())
    }
    pub fn render_in_place(
        &mut self,
        buffer: &mut [f32],
        stereo: bool,
    ) -> Result<(), &'static str> {
        self.validate(buffer.len(), buffer.len(), stereo)?;
        if buffer.is_empty() {
            return Ok(());
        }
        if (5..=9).contains(&self.mode) {
            if stereo {
                self.left_filter.step_bytes = 8;
                self.right_filter.step_bytes = 8;
                let count = buffer.len() / 2;
                self.left_filter.render_in_place(buffer, count)?;
                self.right_filter.render_in_place(&mut buffer[1..], count)?;
            } else {
                let count = buffer.len();
                self.left_filter.render_in_place(buffer, count)?;
            }
        } else if self.mode == 4 {
            let step = if stereo { 2 } else { 1 };
            for i in (0..buffer.len()).step_by(step) {
                let (counter, carry) = self.counter.overflowing_add(self.frequency);
                self.counter = counter;
                if carry {
                    self.held_left = buffer[i];
                    if stereo {
                        self.held_right = buffer[i + 1];
                    }
                }
                buffer[i] = self.held_left;
                if stereo {
                    buffer[i + 1] = self.held_right;
                }
            }
        } else if matches!(self.mode, 1..=3) {
            for value in buffer {
                *value = self.scalar(*value);
            }
        }
        Ok(())
    }
    fn validate(
        &self,
        destination: usize,
        source: usize,
        stereo: bool,
    ) -> Result<(), &'static str> {
        if self.mode > 15 {
            return Err("invalid V2 distortion mode");
        }
        if source < destination || stereo && !destination.is_multiple_of(2) {
            return Err("invalid distortion buffer dimensions");
        }
        Ok(())
    }
    fn scalar(&self, input: f32) -> f32 {
        match self.mode {
            1 => fast_atan(input * self.input_gain + self.offset) * self.output_gain,
            2 => {
                let value = input * self.input_gain + self.offset;
                let value = if value.is_nan() || value > 1. {
                    1.
                } else if value < -1. {
                    -1.
                } else {
                    value
                };
                value * self.output_gain
            }
            3 => {
                let value = integer(input * self.crush_scale)
                    .wrapping_mul(self.crush_step)
                    .clamp(-32767, 32767);
                ((value as u32 ^ self.crush_xor) as i32 as f32) * (1. / 32768.)
            }
            _ => input,
        }
    }
}
