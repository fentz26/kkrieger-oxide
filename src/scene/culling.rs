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
    pub fn distance(&self, point: [f32; 3]) -> f32 {
        self.equation[0] * point[0]
            + self.equation[1] * point[1]
            + self.equation[2] * point[2]
            + self.equation[3]
    }
    pub fn from_points(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Self {
        let d: [f32; 3] = std::array::from_fn(|i| b[i] - a[i]);
        let e: [f32; 3] = std::array::from_fn(|i| c[i] - a[i]);
        let n = [
            d[1] * e[2] - d[2] * e[1],
            d[2] * e[0] - d[0] * e[2],
            d[0] * e[1] - d[1] * e[0],
        ];
        Self::new([n[0], n[1], n[2], -(n[0] * a[0] + n[1] * a[1] + n[2] * a[2])])
    }
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
    pub fn enlarge_to_include(&mut self, point: [f32; 3]) {
        for plane in &mut self.planes[..self.count] {
            let distance = plane.distance(point);
            if distance < 0. {
                plane.equation[3] -= distance;
            }
        }
    }
    pub fn normalize(&mut self) {
        for plane in &mut self.planes[..self.count] {
            let p = plane.equation;
            let length = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
            let scale = (1. / f64::from(length).sqrt()) as f32;
            for value in &mut plane.equation {
                *value *= scale;
            }
        }
    }
    pub fn zfail_volume(
        light: [f32; 3],
        view: Matrix,
        near: f32,
        zoom: [f32; 2],
        viewport: [f32; 4],
    ) -> Self {
        let xs = near / zoom[0];
        let ys = near / zoom[1];
        let points: [[f32; 3]; 4] = std::array::from_fn(|i| {
            let x = xs * viewport[if i & 1 != 0 { 2 } else { 0 }];
            let y = ys * viewport[if i & 2 != 0 { 3 } else { 1 }];
            std::array::from_fn(|axis| {
                let mut value = view[3][axis];
                value += view[0][axis] * x;
                value += view[1][axis] * y;
                value += view[2][axis] * near;
                value
            })
        });
        let mut planes = [Plane::new([0.; 4]); 6];
        planes[0] = Plane::from_points(points[0], points[1], points[2]);
        let distance = planes[0].distance(light);
        let count = if distance.abs() >= 1e-4 {
            for (i, (a, b)) in [(1, 0), (3, 1), (2, 3), (0, 2)].into_iter().enumerate() {
                planes[i + 1] = Plane::from_points(light, points[a], points[b]);
            }
            if distance < 0. {
                for plane in &mut planes[..5] {
                    *plane = Plane::new(plane.equation.map(|v| -v));
                }
            }
            let n: [f32; 3] = std::array::from_fn(|i| {
                let mut value = view[3][i] - light[i];
                value += view[2][i] * near;
                value
            });
            planes[5] = Plane::new([
                n[0],
                n[1],
                n[2],
                -(n[0] * light[0] + n[1] * light[1] + n[2] * light[2]),
            ]);
            6
        } else {
            planes[1] = Plane::new(planes[0].equation.map(|v| -v));
            planes[0].equation[3] += 1e-4;
            planes[1].equation[3] -= 1e-4;
            2
        };
        Self { planes, count }
    }
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
