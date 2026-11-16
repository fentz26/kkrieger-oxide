//! Original engine paint-job bounding boxes and frustum planes.
use crate::mesh::Matrix;
#[derive(Clone, Copy, Debug)]
pub struct Bounds(pub [[f32; 3]; 2]);
impl Bounds {
    pub fn transformed(self, matrix: Matrix) -> Self {
        let minimum: [f32; 3] = std::array::from_fn(|axis| {
            matrix[0][axis] * self.0[0][0]
                + matrix[1][axis] * self.0[0][1]
                + matrix[2][axis] * self.0[0][2]
                + matrix[3][axis]
        });
        let mut result = [minimum; 2];
        for (i, row) in matrix.iter().enumerate().take(3) {
            let extent = self.0[1][i] - self.0[0][i];
            for (axis, value) in row.iter().enumerate().take(3) {
                let delta = value * extent;
                result[if delta < 0. { 0 } else { 1 }][axis] += delta;
            }
        }
        Self(result)
    }
    pub fn intersects_sphere(self, center: [f32; 3], radius: f32) -> bool {
        let mut distance = 0.;
        // Preserve original z/x/y summation order at grazing boundaries.
        for axis in [2, 0, 1] {
            let low = center[axis] - self.0[0][axis];
            if low < 0. {
                distance += low * low;
            } else {
                let high = center[axis] - self.0[1][axis];
                if high > 0. {
                    distance += high * high;
                }
            }
        }
        distance <= radius * radius
    }
    pub fn culled(self, planes: &[Plane]) -> bool {
        planes.iter().any(|plane| {
            let vertex: [f32; 3] = std::array::from_fn(|axis| {
                self.0[usize::from(plane.near_vertex & (1 << axis) != 0)][axis]
            });
            plane.equation[0] * vertex[0]
                + plane.equation[1] * vertex[1]
                + plane.equation[2] * vertex[2]
                + plane.equation[3]
                < -1e-6
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub equation: [f32; 4],
    pub near_vertex: u8,
}
impl Plane {
    pub fn new(equation: [f32; 4]) -> Self {
        let mut near_vertex = 0;
        for (axis, value) in equation.iter().enumerate().take(3) {
            if *value > 0. {
                near_vertex |= 1 << axis;
            }
        }
        Self {
            equation,
            near_vertex,
        }
    }
}
pub struct Frustum {
    pub planes: [Plane; 6],
    pub count: usize,
}
impl Frustum {
    pub fn from_view_projection(matrix: Matrix, viewport: [f32; 4]) -> Self {
        let coefficients = [
            (-viewport[0], 0, 1.),
            (viewport[2], 0, -1.),
            (-viewport[1], 1, 1.),
            (viewport[3], 1, -1.),
            (0., 2, 1.),
            (1., 2, -1.),
        ];
        let planes: [Plane; 6] = coefficients.map(|(a, column, b)| {
            Plane::new(std::array::from_fn(|axis| {
                matrix[axis][3] * a + matrix[axis][column] * b
            }))
        });
        let far = planes[5].equation;
        let count = if far[0] * far[0] + far[1] * far[1] + far[2] * far[2] > 1e-20 {
            6
        } else {
            5
        };
        Self { planes, count }
    }
}
