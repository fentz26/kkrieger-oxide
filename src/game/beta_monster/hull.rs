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

/// Fixed-capacity released collision cell geometry, before dynamic transforms.
#[derive(Clone, Debug)]
pub struct Hull {
    pub planes: [[f32; 4]; 12],
    pub count: usize,
    pub bounds: [[f32; 4]; 2],
    pub center: [f32; 4],
    pub mode: u32,
    pub only_for_shot: bool,
}
impl Hull {
    pub fn from_vertices(vertices: [[f32; 4]; 8], mode: u32) -> Self {
        const ORDER: [[usize; 4]; 6] = [
            [0, 2, 3, 1],
            [2, 6, 7, 3],
            [3, 7, 5, 1],
            [4, 0, 1, 5],
            [2, 0, 4, 6],
            [7, 6, 4, 5],
        ];
        let mut planes = [[0.; 4]; 12];
        let mut count = 0;
        let mut push = |plane: [f32; 4]| {
            planes[count] = plane;
            count += 1;
        };
        for indices in ORDER {
            let [a, b, c, d] = indices.map(|i| vertices[i]);
            if let Some(mut plane) = triangle_plane(a, b, c) {
                let distance = add(dot(plane, d), -f64::from(plane[3]));
                if distance < -f64::from(0.005f32) {
                    if let Some(next) = triangle_plane(b, c, d) {
                        plane = next;
                    }
                    push(plane);
                    if let Some(next) = triangle_plane(d, a, b) {
                        plane = next;
                    }
                } else if distance > f64::from(0.005f32) {
                    push(plane);
                    if let Some(next) = triangle_plane(c, d, a) {
                        plane = next;
                    }
                }
                push(plane);
            } else if let Some(plane) = triangle_plane(c, d, a) {
                // Released fallback selects this triangle; the intervening bcd
                // and final dab calls do not contribute an output plane.
                push(plane);
            }
        }
        for plane in &mut planes[..count] {
            plane[3] = down(add(f64::from(0.005f32), -f64::from(plane[3])));
            if mode & 3 == 1 {
                plane[3] = down(add(f64::from(plane[3]), f64::from(0.005f32)));
            }
        }
        let mut bounds = [vertices[0]; 2];
        let mut center = vertices[0];
        for point in &vertices[1..] {
            for axis in 0..3 {
                center[axis] = down(add(f64::from(center[axis]), f64::from(point[axis])));
                bounds[0][axis] = if bounds[0][axis] < point[axis] {
                    bounds[0][axis]
                } else {
                    point[axis]
                };
                bounds[1][axis] = if bounds[1][axis] > point[axis] {
                    bounds[1][axis]
                } else {
                    point[axis]
                };
            }
        }
        for axis in 0..3 {
            bounds[0][axis] = down(add(f64::from(bounds[0][axis]), -f64::from(0.005f32)));
            bounds[1][axis] = down(add(f64::from(bounds[1][axis]), f64::from(0.005f32)));
            center[axis] = down(mul(f64::from(center[axis]), 0.125));
        }
        center[3] = 1.;
        Self {
            planes,
            count,
            bounds,
            center,
            mode: mode & 0xb,
            only_for_shot: mode & 4 != 0,
        }
    }
}
