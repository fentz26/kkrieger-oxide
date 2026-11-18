//! Released hull plane tests. Their rounding, evaluation order and radius sign
//! differ from the later development collision implementation.
use super::{add, down, mul, rounded_arithmetic};
use crate::collision::Intersection;
fn dot(plane: [f32; 4], value: [f32; 4]) -> f64 {
    add(
        add(
            mul(f64::from(plane[2]), f64::from(value[2])),
            mul(f64::from(plane[1]), f64::from(value[1])),
        ),
        mul(f64::from(plane[0]), f64::from(value[0])),
    )
}
fn divide(a: f64, b: f64) -> f64 {
    let q = a / b;
    rounded_arithmetic(q, (-q).mul_add(b, a) / b)
}
pub fn outside_mask(planes: &[[f32; 4]], point: [f32; 4], radius: f32) -> u32 {
    let mut mask = 0;
    for (index, &p) in planes.iter().enumerate() {
        // OutsideMask's inlined dot uses z,x,y, unlike the shared z,y,x dot.
        let z = mul(f64::from(p[2]), f64::from(point[2]));
        let x = mul(f64::from(p[0]), f64::from(point[0]));
        let y = mul(f64::from(p[1]), f64::from(point[1]));
        let value = add(add(add(add(z, x), y), f64::from(p[3])), f64::from(radius));
        if value < 0. {
            mask |= 1u32.wrapping_shl(index as u32);
        }
    }
    mask
}
pub fn intersect_out(
    planes: &[[f32; 4]],
    point: [f32; 4],
    direction: [f32; 4],
) -> Option<Intersection> {
    let mut best = 0f32;
    let mut result = None;
    for &plane in planes {
        let signed = add(dot(plane, point), f64::from(plane[3]));
        let stored = down(signed);
        if signed < 0. {
            // Released code divides -1 first, then multiplies the stored distance.
            let candidate = mul(divide(-1., dot(plane, direction)), f64::from(stored));
            if candidate < f64::from(best) {
                best = down(candidate);
                result = Some(Intersection {
                    plane,
                    distance: best,
                });
            }
        }
    }
    result
}
pub fn intersect_in(
    planes: &[[f32; 4]],
    point: [f32; 4],
    direction: [f32; 4],
) -> Option<Intersection> {
    let mut best = -1048576f32;
    let mut result = None;
    for &plane in planes {
        let signed = down(add(dot(plane, point), f64::from(plane[3])));
        let determinant = dot(plane, direction);
        if determinant > 1e-16 {
            // FCHS follows the directed division; moving it before FDIV changes bits.
            let candidate = -divide(f64::from(signed), determinant);
            if candidate > f64::from(best) {
                best = down(candidate);
                result = Some(Intersection {
                    plane,
                    distance: best,
                });
            }
        }
    }
    result
}
