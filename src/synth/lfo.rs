//! Original ViruZ II low-frequency oscillator state and patch conversion.
use super::math::integer;
#[derive(Clone, Copy)]
pub struct Parameters {
    pub mode: f32,
    pub sync: f32,
    pub envelope: f32,
    pub rate: f32,
    pub phase: f32,
    pub polarity: f32,
    pub amplitude: f32,
}
#[derive(Clone, Copy, Default)]
pub struct Lfo {
    pub output: f32,
    pub mode: u32,
    pub sync: u32,
    pub envelope: u32,
    pub frequency: u32,
    pub counter: u32,
    pub phase: u32,
    pub gain: f32,
    pub dc: f32,
    pub seed: u32,
    pub last: u32,
}
impl Lfo {
    /// The native initializer obtains this seed with RDTSC.
    pub fn init(&mut self, seed: u32) {
        self.counter = 0;
        self.last = 0;
        self.seed = seed;
    }
    pub fn set(&mut self, p: Parameters) {
        self.mode = integer(p.mode) as u32;
        self.sync = integer(p.sync) as u32;
        self.envelope = integer(p.envelope) as u32;
        let rate = p.rate * (1. / 128.);
        let exponent = (rate - 1.) * 10.;
        let rate = f64::from(exponent).exp2() as f32;
        self.frequency = integer(rate * 2147483648. * 0.5) as u32;
        self.phase = (integer(p.phase * (1. / 128.) * 2147483648.) as u32).wrapping_shl(1);
        let polarity = integer(p.polarity) as u8;
        self.dc = if polarity == 2 {
            -p.amplitude * 0.5
        } else {
            0.
        };
        self.gain = if polarity == 1 {
            -p.amplitude
        } else {
            p.amplitude
        };
    }
    pub fn key_on(&mut self) {
        if self.sync != 0 {
            self.counter = self.phase;
            self.last = u32::MAX;
        }
    }
    pub fn tick(&mut self) -> Result<f32, &'static str> {
        if self.mode > 7 {
            return Err("invalid V2 LFO mode");
        }
        let wave = match self.mode {
            1 => saw(self.counter.wrapping_shl(1) ^ ((self.counter as i32 >> 31) as u32)),
            2 => saw((self.counter as i32 >> 31) as u32),
            3 => super::math::fast_sine(saw(self.counter) * std::f32::consts::TAU) * 0.5 + 0.5,
            4 => {
                if self.counter < self.last {
                    self.seed = self.seed.wrapping_mul(196314165).wrapping_add(907633515);
                }
                self.last = self.counter;
                saw(self.seed)
            }
            _ => saw(self.counter),
        };
        self.output = wave * self.gain + self.dc;
        let (counter, carry) = self.counter.overflowing_add(self.frequency);
        self.counter = if carry && self.envelope != 0 {
            u32::MAX
        } else {
            counter
        };
        Ok(self.output)
    }
}
fn saw(counter: u32) -> f32 {
    f32::from_bits((counter >> 9) | 0x3f800000) - 1.
}
