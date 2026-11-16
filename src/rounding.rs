//! Scalar equivalents of original x87 PC=24, RC=down arithmetic.
//! Keep the process floating-point environment unchanged on every platform.

#[inline]
pub(crate) fn add_down(a: f32, b: f32) -> f32 {
    let rounded = (f64::from(a) + f64::from(b)) as f32;
    if a.is_finite() && b.is_finite() && a == -b {
        return if a.is_sign_negative() || b.is_sign_negative() {
            -0.
        } else {
            0.
        };
    }
    // Subtract the rounded result before adding the smaller operand so that
    // tiny, opposite-sign addends still affect directed rounding.
    let remainder = if a.abs() >= b.abs() {
        (f64::from(a) - f64::from(rounded)) + f64::from(b)
    } else {
        (f64::from(b) - f64::from(rounded)) + f64::from(a)
    };
    if remainder < 0. {
        rounded.next_down()
    } else {
        rounded
    }
}

#[inline]
pub(crate) fn sub_down(a: f32, b: f32) -> f32 {
    add_down(a, -b)
}

#[inline]
pub(crate) fn mul_down(a: f32, b: f32) -> f32 {
    let exact = f64::from(a) * f64::from(b);
    let rounded = exact as f32;
    if f64::from(rounded) > exact {
        rounded.next_down()
    } else {
        rounded
    }
}

#[inline]
pub(crate) fn div_down(a: f32, b: f32) -> f32 {
    let exact = f64::from(a) / f64::from(b);
    let rounded = exact as f32;
    if f64::from(rounded) > exact {
        rounded.next_down()
    } else {
        rounded
    }
}
