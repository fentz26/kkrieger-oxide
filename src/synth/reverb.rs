//! Original ViruZ II stereo Schroeder reverb and low/high-cut state.
use super::rate::Rate;
pub const LENGTHS: [usize; 12] = [
    1309, 1635, 1811, 1926, 220, 74, 1327, 1631, 1833, 1901, 205, 77,
];
const fn offsets() -> [usize; 12] {
    let mut result = [0; 12];
    let mut i = 1;
    while i < 12 {
        result[i] = result[i - 1] + LENGTHS[i - 1];
        i += 1;
    }
    result
}
const OFFSETS: [usize; 12] = offsets();
const HISTORY_SIZE: usize = OFFSETS[11] + LENGTHS[11];
#[derive(Clone, Copy)]
pub struct Parameters {
    pub time: f32,
    pub high_cut: f32,
    pub low_cut: f32,
    pub volume: f32,
}
pub struct Reverb {
    pub gains: [f32; 6],
    pub input_gain: f32,
    pub damping: f32,
    pub low_cut: f32,
    pub positions: [u32; 12],
    pub lowpass: [f32; 8],
    pub highpass: [f32; 2],
    pub history: Box<[f32]>,
}
impl Default for Reverb {
    fn default() -> Self {
        Self {
            gains: [0.; 6],
            input_gain: 0.,
            damping: 0.,
            low_cut: 0.,
            positions: [0; 12],
            lowpass: [0.; 8],
            highpass: [0.; 2],
            history: vec![0.; HISTORY_SIZE].into_boxed_slice(),
        }
    }
}
impl Reverb {
    pub fn init(&mut self) {
        self.gains = [0.; 6];
        self.input_gain = 0.;
        self.damping = 0.;
        self.low_cut = 0.;
        self.reset();
    }
    pub fn reset(&mut self) {
        self.positions = [0; 12];
        self.lowpass = [0.; 8];
        self.highpass = [0.; 2];
        self.history.fill(0.);
    }
    // Keep the original ROM table decimal literals.
    #[allow(clippy::excessive_precision)]
    pub fn set(&mut self, p: Parameters, rate: &Rate) {
        let time = 64. / (p.time + 1.);
        let exponent = (time * time) * rate.linear_frequency;
        self.gains = [
            0.966384599f32,
            0.958186359,
            0.953783929,
            0.950933178,
            0.994260075,
            0.998044717,
        ]
        .map(|base| f64::from(base).powf(f64::from(exponent)) as f32);
        self.damping = (p.high_cut * (1. / 128.)) * rate.linear_frequency;
        self.input_gain = p.volume * (1. / 128.);
        let low = p.low_cut * (1. / 128.);
        let low = low * low;
        self.low_cut = (low * low) * rate.linear_frequency;
    }
    fn advance(&mut self, line: usize) {
        self.positions[line] += 1;
        if self.positions[line] as usize == LENGTHS[line] {
            self.positions[line] = 0;
        }
    }
    // Preserve the assembly's source-first mix addition order.
    #[allow(clippy::assign_op_pattern)]
    pub fn process(&mut self, mix: &mut [[f32; 2]], input: &[f32]) -> Result<(), &'static str> {
        if input.len() < mix.len() {
            return Err("V2 reverb auxiliary input too short");
        }
        if self
            .positions
            .iter()
            .zip(LENGTHS)
            .any(|(&position, length)| position as usize >= length)
        {
            return Err("invalid V2 reverb delay position");
        }
        let mut underflow = false;
        for (output, &input) in mix.iter_mut().zip(input) {
            let input = Float::from(input) * Float::from(self.input_gain)
                + Float::from(f32::from_bits(0x36800000));
            for (side, out) in output.iter_mut().enumerate() {
                let mut sum = Float::from(0.);
                for comb in 0..4 {
                    let line = side * 6 + comb;
                    let index = OFFSETS[line] + self.positions[line] as usize;
                    let delayed = Float::from(self.history[index]) * Float::from(self.gains[comb]);
                    let value = if comb & 1 == 0 {
                        delayed + input
                    } else {
                        delayed - input
                    };
                    let filter = side * 4 + comb;
                    let previous = Float::from(self.lowpass[filter]);
                    let value = (value - previous) * Float::from(self.damping) + previous;
                    let stored = value.store(&mut underflow);
                    self.lowpass[filter] = stored;
                    self.history[index] = stored;
                    if comb == 0 {
                        sum = value;
                    } else {
                        sum = sum + value;
                    }
                    self.advance(line);
                }
                for allpass in 0..2 {
                    let line = side * 6 + 4 + allpass;
                    let index = OFFSETS[line] + self.positions[line] as usize;
                    let delayed = Float::from(self.history[index]);
                    let gain = Float::from(self.gains[4 + allpass]);
                    let value = sum + delayed * gain;
                    self.history[index] = value.store(&mut underflow);
                    sum = delayed - value * gain;
                    self.advance(line);
                }
                let previous = Float::from(self.highpass[side]);
                let high = previous + (sum - previous) * Float::from(self.low_cut);
                self.highpass[side] = high.store(&mut underflow);
                *out = (sum - high + Float::from(*out)).store(&mut underflow);
            }
        }
        if underflow {
            self.reset();
        }
        Ok(())
    }
}

// x87 PC24 rounds arithmetic to 24 significant bits but retains its wide
// exponent range until a float store. Keeping tiny intermediates avoids both
// premature flushes and incorrect tail recovery on exact subnormal stores.
#[derive(Clone, Copy)]
struct Float(f64);
impl From<f32> for Float {
    fn from(value: f32) -> Self {
        Self(f64::from(value))
    }
}
impl Float {
    #[inline]
    fn rounded(value: f64) -> Self {
        let magnitude = value.abs();
        if magnitude >= f64::from(f32::MIN_POSITIVE) && magnitude <= f64::from(f32::MAX) {
            Self(f64::from(value as f32))
        } else if value == 0. || !value.is_finite() {
            Self(value)
        } else {
            let bits = value.to_bits();
            Self(f64::from_bits(
                bits.wrapping_add(0x0fffffff + ((bits >> 29) & 1)) & !0x1fffffff,
            ))
        }
    }
    #[inline]
    fn store(self, underflow: &mut bool) -> f32 {
        let value = self.0 as f32;
        *underflow |= self.0.abs() < f64::from(f32::MIN_POSITIVE) && self.0 != f64::from(value);
        value
    }
}
impl std::ops::Add for Float {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::rounded(self.0 + rhs.0)
    }
}
impl std::ops::Sub for Float {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::rounded(self.0 - rhs.0)
    }
}
impl std::ops::Mul for Float {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::rounded(self.0 * rhs.0)
    }
}
