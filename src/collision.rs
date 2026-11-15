//! Original KKriegerCell geometry, including degenerate and warped faces.
use crate::mesh::Matrix;

const OVERLAP: f32 = 4.010e-3 * 2.;
const ORDER: [[usize; 4]; 6] = [
    [0, 2, 3, 1],
    [2, 6, 7, 3],
    [3, 7, 5, 1],
    [4, 0, 1, 5],
    [2, 0, 4, 6],
    [7, 6, 4, 5],
];

#[derive(Clone, Debug)]
pub struct Cell {
    pub vertices: [[f32; 4]; 8],
    pub planes: Vec<[f32; 4]>,
    pub mode: u32,
    pub only_for_shot: bool,
    pub center: [f32; 4],
    pub bounds: [[f32; 4]; 2],
}

fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn make_plane(a: [f32; 4], b: [f32; 4], c: [f32; 4]) -> Option<[f32; 4]> {
    let d: [f32; 3] = std::array::from_fn(|i| a[i] - b[i]);
    let e: [f32; 3] = std::array::from_fn(|i| b[i] - c[i]);
    let mut v = [
        d[1] * e[2] - d[2] * e[1],
        d[2] * e[0] - d[0] * e[2],
        d[0] * e[1] - d[1] * e[0],
        0.,
    ];
    let length_squared = dot(v, v);
    if (length_squared as f64) < 1e-10 {
        return None;
    }
    // Original sFSqrt takes and returns sF64.
    let length = (length_squared as f64).sqrt();
    for value in &mut v[..3] {
        *value = (*value as f64 / length) as f32;
    }
    v[3] = dot(v, b);
    Some(v)
}

pub fn rotate34(matrix: Matrix, v: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| {
        ((matrix[0][i] * v[0] + matrix[1][i] * v[1]) + matrix[2][i] * v[2]) + matrix[3][i]
    })
}

impl Cell {
    pub fn new(matrix: Matrix, scale: [f32; 3], mode: u32) -> Self {
        let vertices = std::array::from_fn(|i| {
            let mut v = [0.; 4];
            for j in 0..3 {
                v[j] = scale[j] * -(((i >> j) & 1) as f32 - 0.5);
            }
            v[3] = 1.;
            rotate34(matrix, v)
        });
        Self::from_vertices(vertices, mode)
    }

    pub fn from_vertices(vertices: [[f32; 4]; 8], mode: u32) -> Self {
        let mut planes = Vec::with_capacity(12);
        for order in ORDER {
            let [a, b, c, d] = order.map(|i| vertices[i]);
            if let Some(mut plane) = make_plane(a, b, c) {
                let distance = dot(plane, d) - plane[3];
                if distance < -0.005 {
                    if let Some(p) = make_plane(b, c, d) {
                        plane = p;
                    }
                    planes.push(plane);
                    if let Some(p) = make_plane(d, a, b) {
                        plane = p;
                    }
                } else if distance > 0.005 {
                    planes.push(plane);
                    if let Some(p) = make_plane(c, d, a) {
                        plane = p;
                    }
                }
                planes.push(plane);
            } else if let Some(plane) = make_plane(b, c, d)
                .or_else(|| make_plane(c, d, a))
                .or_else(|| make_plane(d, a, b))
            {
                planes.push(plane);
            }
        }
        for plane in &mut planes {
            plane[3] = -plane[3] + OVERLAP;
            if mode & 3 == 1 {
                plane[3] += OVERLAP;
            }
        }
        let mut center = vertices[0];
        let mut bounds = [vertices[0]; 2];
        for v in &vertices[1..] {
            for i in 0..3 {
                center[i] += v[i];
                bounds[0][i] = if bounds[0][i] < v[i] {
                    bounds[0][i]
                } else {
                    v[i]
                };
                bounds[1][i] = if bounds[1][i] > v[i] {
                    bounds[1][i]
                } else {
                    v[i]
                };
            }
        }
        for i in 0..3 {
            bounds[0][i] -= OVERLAP;
            bounds[1][i] += OVERLAP;
            center[i] *= 1. / 8.;
        }
        center[3] = 1.;
        Self {
            vertices,
            planes,
            mode: mode & 3,
            only_for_shot: mode & 4 != 0,
            center,
            bounds,
        }
    }
}
