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

/// Original fastatan, including its signed AH comparison for table selection.
pub fn fast_atan(value: f32) -> f32 {
    let negative = value.is_sign_negative();
    let high = ((((value.to_bits() >> 16) as u16).wrapping_shl(1)) >> 8) as u8 as i8;
    let reciprocal = high == 127;
    let x = if negative { -value } else { value };
    let squared = x * x;
    let (x1, x3, m0, m4, bias) = if reciprocal {
        (
            -0.431597974,
            -1.,
            0.05831938,
            1.,
            std::f64::consts::FRAC_PI_2,
        )
    } else {
        (1., 0.43157974, 1., 0.05831938, 0.)
    };
    let numerator = (f64::from(squared) * x3) as f32;
    let numerator = (f64::from(numerator) + x1) as f32;
    let denominator = (f64::from(squared) * m4) as f32;
    let denominator = (f64::from(denominator) + 0.76443945) as f32;
    let numerator = numerator * x;
    let denominator = denominator * squared;
    let denominator = (f64::from(denominator) + m0) as f32;
    let result = (f64::from(numerator / denominator) + bias) as f32;
    if negative { -result } else { result }
}
