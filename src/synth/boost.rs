//! Original ViruZ II fixed-frequency stereo bass shelving filter.
use super::{math::integer, rate::Rate};
#[derive(Clone, Copy, Default)]
pub struct Boost {
    pub enabled: u32,
    pub a1: f32,
    pub a2: f32,
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub x1: [f32; 2],
    pub x2: [f32; 2],
    pub y1: [f32; 2],
    pub y2: [f32; 2],
}
impl Boost {
    /// syBoostInit deliberately leaves coefficients and delay registers intact.
    pub fn init(&mut self) {}
    pub fn set(&mut self, amount: f32, rate: &Rate) {
        self.enabled = integer(amount) as u32;
        if self.enabled == 0 {
            return;
        }
        let a = f64::from(amount * (1. / 128.)).exp2() as f32;
        let minus = a - 1.;
        let plus = a + 1.;
        let beta = f64::from((a * a + 1.) - minus * minus).sqrt() as f32;
        let bs = beta * rate.boost_sine;
        let c_plus = plus * rate.boost_cosine;
        let c_minus = minus * rate.boost_cosine;
        let inverse = 1. / ((bs + c_minus) + plus);
        let b1 = minus - c_plus;
        self.b1 = ((b1 + b1) * a) * inverse;
        let a1 = c_plus + minus;
        self.a1 = -(a1 + a1) * inverse;
        self.a2 = ((c_minus + plus) - bs) * inverse;
        let a_inverse = a * inverse;
        let base = plus - c_minus;
        self.b0 = (base + bs) * a_inverse;
        self.b2 = (base - bs) * a_inverse;
    }
    pub fn render(&mut self, audio: &mut [[f32; 2]]) {
        // The renderer tests only the low byte of the rounded enable value.
        if self.enabled as u8 == 0 {
            return;
        }
        for side in 0..2 {
            let (mut x1, mut x2) = (self.x1[side], self.x2[side]);
            let (mut y1, mut y2) = (self.y1[side], self.y2[side]);
            for frame in audio.iter_mut() {
                let older = x2 * self.b2 - y2 * self.a2;
                let recent = x1 * self.b1 - y1 * self.a1;
                let input = frame[side] + f32::from_bits(0x36800000);
                let output = input * self.b0 + (recent + older);
                frame[side] = output;
                x2 = x1;
                x1 = input;
                y2 = y1;
                y1 = output;
            }
            self.x1[side] = x1;
            self.x2[side] = x2;
            self.y1[side] = y1;
            self.y2[side] = y2;
        }
    }
}
