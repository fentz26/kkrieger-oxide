//! Released travel normalization and first-contact projection. Subsequent casts
//! and two-plane/three-plane response remain with the collision driver.
use super::{add, down, mul, rounded_arithmetic};
const MIN_TRAVEL_SQUARED: f64 = 1e-16;
const SEPARATION: f32 = f32::from_bits(0x38d1b717);
fn dot(a: [f32; 3], b: [f32; 3]) -> f64 {
    let a = a.map(f64::from);
    let b = b.map(f64::from);
    add(add(mul(a[2], b[2]), mul(a[1], b[1])), mul(a[0], b[0]))
}
fn difference(a: [f32; 4], b: [f32; 4]) -> [f32; 3] {
    std::array::from_fn(|i| down(add(f64::from(a[i]), -f64::from(b[i]))))
}
fn scale_add(mut v: [f32; 4], axis: [f32; 3], factor: f32) -> [f32; 4] {
    for i in 0..3 {
        v[i] = down(add(
            mul(f64::from(axis[i]), f64::from(factor)),
            f64::from(v[i]),
        ));
    }
    v
}
#[derive(Clone, Debug, PartialEq)]
pub struct Travel {
    pub origin: [f32; 4],
    pub target: [f32; 4],
    pub direction: [f32; 3],
    pub inverse_length: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FirstSlide {
    /// Contact point before the separation offset (also used by shot events).
    pub impact: [f32; 4],
    pub travel: Travel,
}
impl Travel {
    /// None selects the collision driver's original short-travel fallback;
    /// it does not mean that the proposed target should be committed.
    pub fn new(origin: [f32; 4], target: [f32; 4]) -> Option<Self> {
        let delta = difference(target, origin);
        let squared = dot(delta, delta);
        if squared < MIN_TRAVEL_SQUARED {
            return None;
        }
        let root = squared.sqrt();
        let root = rounded_arithmetic(root, (-root).mul_add(root, squared));
        let reciprocal = 1. / root;
        let inverse_length = down(rounded_arithmetic(
            reciprocal,
            (-reciprocal).mul_add(root, 1.) / root,
        ));
        let direction = delta.map(|v| down(mul(f64::from(v), f64::from(inverse_length))));
        Some(Self {
            origin,
            target,
            direction,
            inverse_length,
        })
    }
    pub fn first_contact(&self, plane: [f32; 4], distance: f32) -> FirstSlide {
        let normal: [f32; 3] = plane[..3].try_into().unwrap();
        let impact = scale_add(self.target, self.direction, distance);
        let origin = scale_add(impact, normal, SEPARATION);
        let signed_distance = add(
            dot(normal, self.target[..3].try_into().unwrap()),
            f64::from(plane[3]),
        );
        let projection = down(add(f64::from(SEPARATION), -signed_distance));
        let target = scale_add(self.target, normal, projection);
        let delta = difference(target, origin);
        let direction = if dot(delta, delta) > MIN_TRAVEL_SQUARED {
            delta.map(|v| down(mul(f64::from(v), f64::from(self.inverse_length))))
        } else {
            self.direction
        };
        FirstSlide {
            impact,
            travel: Self {
                origin,
                target,
                direction,
                inverse_length: self.inverse_length,
            },
        }
    }
}
