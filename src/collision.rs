//! Original KKriegerCell geometry, including degenerate and warped faces.
use crate::mesh::Matrix;
pub mod connectivity;
pub mod faces;
pub mod world;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Intersection {
    pub plane: [f32; 4],
    pub distance: f32,
}

impl Cell {
    pub fn reject_plane(&self, a: [f32; 4], b: [f32; 4]) -> bool {
        self.planes
            .iter()
            .any(|&plane| dot(plane, a) + plane[3] < 0. && dot(plane, b) + plane[3] < 0.)
    }

    pub fn outside_mask(&self, point: [f32; 4], radius: f32) -> u32 {
        let mut mask = 0;
        for (i, &plane) in self.planes.iter().enumerate() {
            if dot(plane, point) + plane[3] - radius < 0. {
                mask |= 1 << i;
            }
        }
        mask
    }

    pub fn approximate_distance(&self, point: [f32; 4]) -> f32 {
        let mut result = 10000.;
        for &plane in &self.planes {
            let distance = dot(point, plane) + plane[3];
            if distance < result {
                result = distance;
            }
        }
        -result
    }

    /// Original IntersectOut retains negative distances and parallel-plane
    /// divisions. These conventions are consumed by the original CollideRay.
    pub fn intersect_out(
        &self,
        point: [f32; 4],
        direction: [f32; 4],
        radius: f32,
    ) -> Option<Intersection> {
        let mut best = 0.;
        let mut result = None;
        for &plane in &self.planes {
            let distance = dot(plane, point) + plane[3] - radius;
            if distance < 0. {
                let distance = -distance / dot(plane, direction);
                if distance < best {
                    best = distance;
                    result = Some(Intersection { plane, distance });
                }
            }
        }
        result
    }

    pub fn intersect_in(
        &self,
        point: [f32; 4],
        direction: [f32; 4],
        radius: f32,
    ) -> Option<Intersection> {
        let mut best = -1048576.;
        let mut result = None;
        for &plane in &self.planes {
            let distance = dot(plane, point) + plane[3] - radius;
            let determinant = dot(plane, direction);
            // The C++ literal is double precision.
            if determinant as f64 > 1.0e-16 {
                let distance = -distance / determinant;
                if distance > best {
                    best = distance;
                    result = Some(Intersection { plane, distance });
                }
            }
        }
        result
    }
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

/// Persistent geometry and transforms used by KKriegerCellDynamic.
#[derive(Clone, Debug)]
pub struct DynamicCell {
    pub cell: Cell,
    pub original_planes: Vec<[f32; 4]>,
    pub original_vertices: [[f32; 4]; 8],
    pub confirmed_matrix: Matrix,
    pub current_matrix: Matrix,
    pub matrix0: Matrix,
    pub matrix1: Matrix,
    pub blocked: i32,
    pub monster: Option<usize>,
}

fn rotate3(matrix: Matrix, v: [f32; 4]) -> [f32; 4] {
    let mut result = [0.; 4];
    for i in 0..3 {
        result[i] = (matrix[0][i] * v[0] + matrix[1][i] * v[1]) + matrix[2][i] * v[2];
    }
    result
}

fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
        0.,
    ]
}

fn unit3(v: &mut [f32; 4]) {
    let scale = (1. / (dot(*v, *v) as f64).sqrt()) as f32;
    for x in &mut v[..3] {
        *x *= scale;
    }
}

/// Original sMatrix::Lin4: interpolate two axes, then orthonormalize.
/// In particular, this discards scale and derives the third axis by crossing.
pub fn interpolate_matrix(a: Matrix, b: Matrix, fade: f32) -> Matrix {
    let mut matrix = crate::mesh::identity();
    for row in [0, 1, 3] {
        for i in 0..3 {
            matrix[row][i] = a[row][i] + (b[row][i] - a[row][i]) * fade;
        }
    }
    matrix[2] = cross(matrix[0], matrix[1]);
    unit3(&mut matrix[2]);
    matrix[0] = cross(matrix[1], matrix[2]);
    unit3(&mut matrix[0]);
    unit3(&mut matrix[1]);
    matrix
}

impl DynamicCell {
    pub fn new(mut cell: Cell, matrix: Matrix) -> Self {
        let original_planes = cell.planes.clone();
        let original_vertices = cell.vertices;
        cell.mode |= 0x20;
        Self {
            cell,
            original_planes,
            original_vertices,
            confirmed_matrix: matrix,
            current_matrix: matrix,
            matrix0: matrix,
            matrix1: matrix,
            blocked: 0,
            monster: None,
        }
    }

    /// Original ApplyMatrix leaves the obsolete bounding box untouched.
    pub fn apply_matrix(&mut self, matrix: Matrix) {
        for (plane, original) in self.cell.planes.iter_mut().zip(&self.original_planes) {
            let mut result = rotate3(matrix, *original);
            result[3] = original[3] - dot(result, matrix[3]);
            let length = (dot(result, result) as f64).sqrt() as f32;
            let scale = 1. / length;
            for x in &mut result {
                *x *= scale;
            }
            *plane = result;
        }
        for (vertex, original) in self.cell.vertices.iter_mut().zip(self.original_vertices) {
            let v = rotate3(matrix, original);
            for i in 0..3 {
                vertex[i] = v[i] + matrix[3][i];
            }
        }
        self.cell.center = matrix[3];
        self.current_matrix = matrix;
        self.blocked = 0;
    }

    /// The dynamic-cell phase of KKriegerGame::Physic.
    pub fn physics(&mut self, fade: f32) {
        let matrix = interpolate_matrix(self.matrix0, self.matrix1, fade);
        self.apply_matrix(matrix);
        self.confirmed_matrix = self.current_matrix;
        self.blocked = 0;
        self.cell.mode &= !0x20;
    }
}
