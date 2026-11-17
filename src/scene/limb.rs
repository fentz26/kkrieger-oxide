//! Original two-segment Scene_Limb inverse kinematics.
use crate::mesh::{Matrix, identity};
pub struct Joint {
    pub transform: Matrix,
    pub other: Matrix,
}
pub fn joints(
    parent: Matrix,
    mut position: [f32; 3],
    mut direction: [f32; 3],
    a: f32,
    b: f32,
    flags: u32,
) -> [Joint; 2] {
    if flags & 2 == 0 {
        for axis in 0..3 {
            position[axis] -= parent[3][axis];
        }
        direction = std::array::from_fn(|axis| {
            parent[0][axis] * direction[0]
                + parent[1][axis] * direction[1]
                + parent[2][axis] * direction[2]
        });
    }
    let j = position.map(|v| -v);
    let k = cross(direction, j);
    let i = cross(j, k);
    let c = (f64::from(dot(j, j)).sqrt()) as f32;
    let i = unit(i);
    let j = unit(j);
    let k = unit(k);
    let t = (c * c + a * a - b * b) / (2. * c);
    let squared = (a - t) * (a + t);
    let s = if squared <= 0. {
        0.
    } else {
        f64::from(squared).sqrt() as f32
    };
    let mut first = identity();
    first[2] = vector(k);
    first[1] = vector(unit(std::array::from_fn(|axis| j[axis] * t + i[axis] * s)));
    first[1][3] = 0. * t;
    first[0] = vector(cross(first[1][..3].try_into().unwrap(), k));
    let mut second;
    if a <= t && flags & 1 == 0 {
        second = first;
        for axis in 0..3 {
            second[3][axis] += j[axis] * (-a - b);
        }
    } else {
        second = identity();
        second[2] = vector(k);
        second[1] = vector(unit(std::array::from_fn(|axis| {
            j[axis] * (c - t) + i[axis] * -s
        })));
        second[1][3] = 0. * (c - t);
        second[0] = vector(cross(second[1][..3].try_into().unwrap(), k));
        second[3] = [position[0], position[1], position[2], 1.];
    }
    let transform = |mut matrix: Matrix| {
        if flags & 2 != 0 {
            super::exec::multiply(matrix, parent)
        } else {
            for (value, offset) in matrix[3][..3].iter_mut().zip(parent[3]) {
                *value += offset;
            }
            matrix
        }
    };
    [
        Joint {
            transform: transform(first),
            other: second,
        },
        Joint {
            transform: transform(second),
            other: first,
        },
    ]
}
fn vector(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: [f32; 3]) -> [f32; 3] {
    let scale = (1. / f64::from(dot(v, v)).sqrt()) as f32;
    v.map(|value| value * scale)
}
