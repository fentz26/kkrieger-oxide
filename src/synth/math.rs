//! Original x87 fastsinrc range reduction and minimax sine polynomial.
pub fn fast_sine(value: f32) -> f32 {
    let mut x = value % std::f32::consts::TAU;
    if x > std::f32::consts::PI * 1.5 {
        x -= std::f32::consts::TAU;
    } else if x > std::f32::consts::FRAC_PI_2 {
        x = (std::f64::consts::PI - f64::from(x)) as f32;
    }
    sine_polynomial(x)
}

pub(crate) fn sine_polynomial(x: f32) -> f32 {
    let squared = x * x;
    let c = (-0.00018542f64 * f64::from(squared)) as f32;
    let b = (f64::from(c) + 0.0083143f64) as f32;
    let b = b * squared;
    let a = (f64::from(b) - 0.16666f64) as f32;
    ((a * squared) + 1.) * x
}

pub(crate) fn integer(value: f32) -> i32 {
    let value = value.round_ties_even();
    if !(-2147483648.0..2147483648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}
