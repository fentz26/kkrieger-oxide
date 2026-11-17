//! Original ViruZ II stereo compressor, with shared caller-owned level scratch.
use super::math::integer;
#[derive(Clone, Copy)]
pub struct Parameters {
    pub mode: f32,
    pub stereo: f32,
    pub automatic_gain: f32,
    pub lookahead: f32,
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
    pub output_gain: f32,
}
pub struct Compressor {
    pub mode: u32,
    pub old_mode: u32,
    pub input_gain: f32,
    pub ratio: f32,
    pub output_gain: f32,
    pub attack: f32,
    pub release: f32,
    pub lookahead_length: u32,
    pub lookahead_position: u32,
    pub current_gain: [f32; 2],
    pub peak: [f32; 2],
    pub rms_position: u32,
    pub rms_value: [f32; 2],
    pub lookahead: Box<[[f32; 2]]>,
    pub rms_history: Box<[f32]>,
}
impl Default for Compressor {
    fn default() -> Self {
        Self {
            mode: 0,
            old_mode: 0,
            input_gain: 0.,
            ratio: 0.,
            output_gain: 0.,
            attack: 0.,
            release: 0.,
            lookahead_length: 0,
            lookahead_position: 0,
            current_gain: [0.; 2],
            peak: [0.; 2],
            rms_position: 0,
            rms_value: [0.; 2],
            lookahead: vec![[0.; 2]; 5700].into_boxed_slice(),
            rms_history: vec![0.; 16384].into_boxed_slice(),
        }
    }
}
impl Compressor {
    pub fn init(&mut self) {
        self.mode = (self.mode & 0xffffff00) | 2;
    }
    pub fn set(&mut self, p: Parameters) -> Result<(), &'static str> {
        // The native delay length uses a fixed 44.1 samples/ms at every rate.
        let length = integer(p.lookahead * 44.1) as u32;
        if length >= 5700 {
            return Err("V2 compressor lookahead exceeds delay storage");
        }
        self.mode = ((integer(p.mode) as u32).wrapping_sub(1) & 5)
            .wrapping_add((integer(p.stereo) as u32).wrapping_mul(2));
        if self.mode != self.old_mode {
            self.old_mode = self.mode;
            self.peak = [0.; 2];
            self.rms_value = [0.; 2];
            self.rms_history.fill(0.);
            self.current_gain = [1.; 2];
        }
        self.lookahead_length = length;
        let threshold = f64::from((p.threshold * (1. / 128.) - 1.) * 10.).exp2() as f32;
        let threshold = threshold + threshold;
        let threshold = threshold + threshold;
        let threshold = threshold + threshold;
        self.input_gain = 1. / threshold;
        let output = if integer(p.automatic_gain) != 0 {
            1.
        } else {
            threshold
        };
        self.output_gain = output * (f64::from((p.output_gain - 64.) * (1. / 16.)).exp2() as f32);
        self.ratio = p.ratio * (1. / 128.);
        self.attack = f64::from((p.attack * (1. / 128.)) * -12.).exp2() as f32;
        self.release = f64::from((p.release * (1. / 128.)) * -16.).exp2() as f32;
        Ok(())
    }
    fn detect(&mut self, input: &[[f32; 2]], levels: &mut [[f32; 2]]) {
        let dc = f32::from_bits(0x36800000);
        let stereo = self.mode & 2 != 0;
        if self.mode & 1 == 0 {
            for (frame, level) in input.iter().zip(levels) {
                if stereo {
                    for side in 0..2 {
                        let decayed = self.peak[side] * 0.9998 + dc;
                        self.peak[side] = f32::from_bits(
                            (frame[side].to_bits() & 0x7fffffff).max(decayed.to_bits()),
                        );
                        level[side] = self.peak[side] * self.input_gain;
                    }
                } else {
                    let value = (frame[0] + frame[1]) * 0.5;
                    let decayed = self.peak[0] * 0.9998 + dc;
                    self.peak[0] =
                        f32::from_bits((value.to_bits() & 0x7fffffff).max(decayed.to_bits()));
                    *level = [self.peak[0] * self.input_gain; 2];
                }
            }
        } else {
            for (frame, level) in input.iter().zip(levels) {
                let index = self.rms_position as usize;
                if stereo {
                    for side in 0..2 {
                        let value = frame[side] * frame[side];
                        let sum = (self.rms_value[side] - self.rms_history[index * 2 + side]) + dc;
                        self.rms_value[side] = sum + value;
                        self.rms_history[index * 2 + side] = value;
                        level[side] = ((f64::from(self.rms_value[side]).sqrt() as f32)
                            * 0.011048543)
                            * self.input_gain;
                    }
                } else {
                    let sum = self.rms_value[0] - self.rms_history[index];
                    let value = (frame[0] + frame[1]) * 0.5 + dc;
                    let value = value * value;
                    self.rms_value[0] = sum + value;
                    self.rms_history[index] = value;
                    let level_value = ((f64::from(self.rms_value[0]).sqrt() as f32) * 0.011048543)
                        * self.input_gain;
                    *level = [level_value; 2];
                }
                self.rms_position = (self.rms_position + 1) & 8191;
            }
        }
    }
    pub fn render(
        &mut self,
        audio: &mut [[f32; 2]],
        levels: &mut [[f32; 2]],
    ) -> Result<(), &'static str> {
        if self.mode & 4 != 0 {
            return Ok(());
        }
        if levels.len() < audio.len() {
            return Err("V2 compressor level buffer too short");
        }
        if self.lookahead_length >= 5700
            || self.lookahead_position >= 5700
            || self.rms_position >= 8192
        {
            return Err("invalid V2 compressor delay position");
        }
        self.detect(audio, &mut levels[..audio.len()]);
        let start = self.lookahead_position;
        for side in 0..2 {
            let mut position = start;
            let mut gain = self.current_gain[side];
            for (frame, level) in audio.iter_mut().zip(levels.iter()) {
                let value = self.lookahead[position as usize][side];
                self.lookahead[position as usize][side] = frame[side] * self.input_gain;
                let value = value * self.output_gain;
                position += 1;
                if position > self.lookahead_length {
                    position = 0;
                }
                let target = if level[side].to_bits() < 0x3f800000 {
                    1.
                } else {
                    1. / (1. + self.ratio * (level[side] - 1.))
                };
                let speed = if target.to_bits() < gain.to_bits() {
                    self.attack
                } else {
                    self.release
                };
                gain += speed * (target - gain);
                frame[side] = value * gain;
            }
            self.current_gain[side] = gain;
            self.lookahead_position = position;
        }
        Ok(())
    }
}
