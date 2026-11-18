//! Released travel normalization and first/second-contact projection.
//! World casts, final-contact fallback and driver integration remain separate.
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
fn project(target: [f32; 4], plane: [f32; 4]) -> [f32; 4] {
    let normal = [plane[0], plane[1], plane[2]];
    let signed_distance = add(
        dot(normal, [target[0], target[1], target[2]]),
        f64::from(plane[3]),
    );
    let factor = down(add(f64::from(SEPARATION), -signed_distance));
    scale_add(target, normal, factor)
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
        let target = project(self.target, plane);
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

#[derive(Clone, Debug, PartialEq)]
pub struct SecondSlide {
    /// The driver copies this into both Pos and OldPos for opposing contacts.
    pub reset_position: Option<[f32; 4]>,
    /// Direction includes the extra inverse-length scale used for cast three.
    pub travel: Travel,
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 3] {
    std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        down(add(
            mul(f64::from(a[j]), f64::from(b[k])),
            -mul(f64::from(a[k]), f64::from(b[j])),
        ))
    })
}
fn triple(a: f32, b: f32, c: f32) -> f64 {
    mul(mul(f64::from(a), f64::from(b)), f64::from(c))
}
fn reciprocal(value: f64) -> f32 {
    let q = 1. / value;
    down(rounded_arithmetic(q, (-q).mul_add(value, 1.) / value))
}
fn scale(mut point: [f32; 4], factor: f32) -> [f32; 4] {
    for value in &mut point[..3] {
        *value = down(mul(f64::from(*value), f64::from(factor)));
    }
    point
}
impl Travel {
    pub fn second_contact(
        &self,
        first_plane: [f32; 4],
        second_plane: [f32; 4],
        distance: f32,
        old_collision: [f32; 4],
    ) -> SecondSlide {
        let alignment = down(dot(
            first_plane[..3].try_into().unwrap(),
            second_plane[..3].try_into().unwrap(),
        ));
        let mut origin = scale_add(self.target, self.direction, distance);
        let mut target = self.target;
        let mut reset_position = None;
        if alignment < f32::from_bits(0xbf666666) {
            target = old_collision;
            reset_position = Some(old_collision);
        } else if alignment > SEPARATION {
            origin = scale_add(
                origin,
                [second_plane[0], second_plane[1], second_plane[2]],
                SEPARATION,
            );
            target = project(target, second_plane);
        } else {
            let mut a = first_plane;
            let mut b = second_plane;
            a[3] = down(add(f64::from(a[3]), -f64::from(SEPARATION)));
            b[3] = down(add(f64::from(b[3]), -f64::from(SEPARATION)));
            let c = cross(first_plane, second_plane);
            let c4 = [c[0], c[1], c[2], 0.];
            let ac = cross(b, c4);
            let bc = cross(c4, a);
            let base = scale_add(scale_add([0.; 4], ac, -a[3]), bc, -b[3]);
            let ab = cross(a, b);
            let mut det = add(triple(c[1], b[0], a[2]), triple(c[0], b[2], a[1]));
            det = add(det, triple(c[2], b[1], a[0]));
            det = add(det, -triple(c[1], b[2], a[0]));
            det = add(det, -triple(b[0], a[1], c[2]));
            det = add(det, -triple(a[2], c[0], b[1]));
            let inverse = reciprocal(det);
            target = scale(
                scale_add(base, ab, down(dot(c, target[..3].try_into().unwrap()))),
                inverse,
            );
            origin = scale(
                scale_add(base, ab, down(dot(c, origin[..3].try_into().unwrap()))),
                inverse,
            );
        }
        let delta = difference(target, origin);
        let direction = if dot(delta, delta) > MIN_TRAVEL_SQUARED {
            delta.map(|v| down(mul(f64::from(v), f64::from(self.inverse_length))))
        } else {
            self.direction
        };
        let direction = direction.map(|v| down(mul(f64::from(v), f64::from(self.inverse_length))));
        SecondSlide {
            reset_position,
            travel: Self {
                origin,
                target,
                direction,
                inverse_length: self.inverse_length,
            },
        }
    }
}
