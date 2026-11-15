//! Original CheckSphereVsEdge/Face and soft sphere response.
use super::{cross, dot, unit3};

fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.]
}
fn length(v: [f32; 4]) -> f32 {
    (dot(v, v) as f64).sqrt() as f32
}
fn point(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.]
}

/// The output can change on a false result: the source writes the projected
/// edge point before its final distance check. Face traversal relies on this.
pub fn check_edge(sphere: [f32; 4], a: [f32; 4], b: [f32; 4], nearest: &mut [f32; 3]) -> bool {
    let mut unit = sub(b, a);
    let len = length(unit);
    if len < 4.010e-3 {
        return false;
    }
    let scale = 1. / len;
    for v in &mut unit[..3] {
        *v *= scale;
    }
    let to_sphere = sub(sphere, a);
    let distance = dot(unit, to_sphere);
    if distance < 0. {
        if length(to_sphere) > sphere[3] {
            return false;
        }
        *nearest = [a[0], a[1], a[2]];
        return true;
    }
    if distance > len {
        return false;
    }
    *nearest = std::array::from_fn(|i| unit[i] * distance + a[i]);
    let outside = length(sub(sphere, point(*nearest))) > sphere[3];
    !outside
}

pub fn check_face(sphere: [f32; 4], vertices: &[[f32; 4]], nearest: &mut [f32; 3]) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    let mut normal = cross(sub(vertices[2], vertices[1]), sub(vertices[0], vertices[1]));
    if length(normal) <= 4.010e-3 {
        return false;
    }
    unit3(&mut normal);
    let distance = dot(normal, sub(sphere, vertices[0]));
    if distance.abs() > sphere[3] {
        return false;
    }
    let mut copy = sphere;
    let mut result = false;
    let mut inside = true;
    for i in 0..vertices.len() {
        let next = vertices[(i + 1) % vertices.len()];
        let edge_normal = cross(sub(next, vertices[i]), normal);
        if dot(sub(sphere, vertices[i]), edge_normal) > 0. {
            inside = false;
            if check_edge(copy, vertices[i], next, nearest) {
                result = true;
                copy[3] = length(sub(copy, point(*nearest)));
            }
        }
    }
    if !inside {
        return result;
    }
    *nearest = std::array::from_fn(|i| normal[i] * -distance + sphere[i]);
    true
}

/// Radius is sphere.w, and shrinks as each face pushes the center away.
pub fn collide_face(sphere: &mut [f32; 4], vertices: &[[f32; 4]]) {
    let mut nearest = [0.; 3];
    if check_face(*sphere, vertices, &mut nearest) {
        let shift = sub(*sphere, point(nearest));
        let len = length(shift);
        let distance = (sphere[3] - len) * 0.5;
        let scale = distance / len;
        for i in 0..3 {
            sphere[i] += shift[i] * scale;
        }
        sphere[3] -= distance;
    }
}
