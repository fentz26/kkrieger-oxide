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

/// Released FindCell returns the first containing cell in registry order.
/// Boundary points are inside; an empty registry returns None.
pub fn find_cell<'a>(
    cells: impl IntoIterator<Item = (usize, &'a [[f32; 4]])>,
    point: [f32; 4],
) -> Option<usize> {
    cells
        .into_iter()
        .find_map(|(id, planes)| (outside_mask(planes, point, 0.) == 0).then_some(id))
}

/// Released MakePlane result before cell overlap adjustments. The fourth word
/// is dot(normal,b), not the negated distance consumed by containment tests.
pub fn triangle_plane(a: [f32; 4], b: [f32; 4], c: [f32; 4]) -> Option<[f32; 4]> {
    let d: [f32; 3] = std::array::from_fn(|i| down(add(f64::from(a[i]), -f64::from(b[i]))));
    let e: [f32; 3] = std::array::from_fn(|i| down(add(f64::from(b[i]), -f64::from(c[i]))));
    let n: [f32; 3] = std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        down(add(
            mul(f64::from(d[j]), f64::from(e[k])),
            -mul(f64::from(d[k]), f64::from(e[j])),
        ))
    });
    let n4 = [n[0], n[1], n[2], 0.];
    let squared = dot(n4, n4);
    if squared < 1e-10 {
        return None;
    }
    let root = squared.sqrt();
    let root = rounded_arithmetic(root, (-root).mul_add(root, squared));
    // The reciprocal remains on the x87 stack; it is not first stored to f32.
    let inverse = divide(1., root);
    let n = n.map(|v| down(mul(f64::from(v), inverse)));
    let n = [n[0], n[1], n[2], 0.];
    Some([n[0], n[1], n[2], down(dot(n, b))])
}
