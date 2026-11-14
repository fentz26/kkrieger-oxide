use super::{Matrix, Mesh, Selection, euler_matrix, identity, srt_matrix};

fn rotate(matrix: &Matrix, v: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| {
        v[0] * matrix[0][i] + v[1] * matrix[1][i] + v[2] * matrix[2][i] + matrix[3][i]
    })
}
fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if length > 1e-20 {
        let scale = 1. / length.sqrt();
        v.map(|v| v * scale)
    } else {
        [1., 0., 0.]
    }
}

impl Mesh {
    pub fn perlin(&mut self, matrix: &Matrix, amplitude: [f32; 3], noise: &crate::random::Noise) {
        use crate::rounding::{add_down as add, mul_down as mul};
        for vertex in &mut self.vertices {
            if !vertex.element.selected {
                continue;
            }
            let position = vertex.attributes[0];
            let point = std::array::from_fn(|axis| {
                let mut value = mul(matrix[0][axis], position[0]);
                value = add(value, mul(matrix[1][axis], position[1]));
                value = add(value, mul(matrix[2][axis], position[2]));
                add(value, matrix[3][axis])
            });
            let displacement = noise.vector3_directed(point);
            for axis in 0..3 {
                vertex.attributes[0][axis] =
                    add(position[axis], mul(displacement[axis], amplitude[axis]));
            }
        }
    }

    pub fn bend_between(
        &mut self,
        first: [f32; 9],
        second: [f32; 9],
        direction: [f32; 2],
        range: [f32; 2],
        mode: u32,
    ) {
        let first = srt_matrix(&first);
        let second = srt_matrix(&second);
        let direction = euler_matrix([
            direction[0] * std::f32::consts::TAU,
            0.,
            direction[1] * std::f32::consts::TAU,
        ]);
        let coordinate =
            |v: [f32; 4]| v[0] * direction[0][1] + v[1] * direction[1][1] + v[2] * direction[2][1];
        let (low, high) = if mode & 2 != 0 {
            let mut low = 1e30_f32;
            let mut high = -1e30_f32;
            for v in &self.vertices {
                if v.element.selected {
                    let y = coordinate(v.attributes[0]);
                    low = low.min(y);
                    high = high.max(y);
                }
            }
            (low, high)
        } else {
            (range[0], range[1])
        };
        let scale = 1. / (high - low);
        for vertex in &mut self.vertices {
            if !vertex.element.selected {
                continue;
            }
            let mut t = ((coordinate(vertex.attributes[0]) - low) * scale).clamp(0., 1.);
            t = match mode & 5 {
                1 => t * t * (3. - 2. * t),
                4 => 1. - (t - 0.5).abs() * 2.,
                5 => t * t * (16. + t * (16. * t - 32.)),
                _ => t,
            };
            let a = rotate(&first, vertex.attributes[0]);
            let b = rotate(&second, vertex.attributes[0]);
            for axis in 0..3 {
                vertex.attributes[0][axis] = a[axis] + (b[axis] - a[axis]) * t;
            }
        }
    }

    pub fn bend_curve(&mut self, center: [f32; 3], rotation: [f32; 3], length: f32, angle: f32) {
        let mut transform = srt_matrix(&[
            1.,
            1.,
            1.,
            rotation[0],
            rotation[1],
            rotation[2],
            0.,
            0.,
            0.,
        ]);
        transform[3] = [-center[0], -center[1], -center[2], 1.];
        let mut inverse = identity();
        for column in 0..3 {
            for row in 0..3 {
                inverse[column][row] = transform[row][column];
            }
        }
        let translation = std::array::from_fn::<_, 3, _>(|axis| {
            center[0] * inverse[0][axis]
                + center[1] * inverse[1][axis]
                + center[2] * inverse[2][axis]
        });
        inverse[3][..3].copy_from_slice(&translation);
        let angle = angle * std::f32::consts::TAU;
        for vertex in &mut self.vertices {
            let mut position = rotate(&transform, vertex.attributes[0]);
            let y = position[1];
            if y >= 0. {
                position[1] -= y.min(length);
            }
            let t = (y / length).clamp(0., 1.) * angle;
            let sin = t.sin();
            let cos = t.cos();
            let x = position[0];
            let y = position[1];
            position[0] = cos * x - sin * y;
            position[1] = sin * x + cos * y;
            vertex.attributes[0] = rotate(&inverse, position);
        }
        self.got_normals = false;
    }

    pub fn bevel(
        &mut self,
        elevate: f32,
        pull_in: f32,
        mode: i32,
        mask: u32,
        selection: Selection,
    ) {
        self.extrude(mask, selection, mode);
        for half in 0..self.edges.len() * 2 {
            self.edges[half >> 1].temp[half & 1] = if self.is_border_edge(half, mode) {
                0
            } else {
                -1
            };
        }
        for half in 0..self.edges.len() * 2 {
            if self.edges[half >> 1].temp[half & 1] != -1 {
                let mut edge = self.previous_vertex_edge(half);
                while self.edges[edge >> 1].temp[!edge & 1] == -1 {
                    assert_ne!(edge, half);
                    edge = self.previous_vertex_edge(edge);
                }
                self.edges[half >> 1].temp[half & 1] = (edge ^ 1) as i32;
            }
        }
        for half in 0..self.edges.len() * 2 {
            let edge = self.edges[half >> 1].temp[half & 1];
            if edge == -1 {
                continue;
            }
            let edge = edge as usize;
            let first = self.vertex_id(edge);
            let mut vertex = first;
            loop {
                let previous = self.vertex_id(half);
                let next = self.vertex_id(self.edges[edge >> 1].temp[edge & 1] as usize);
                let copy = self.add_copied_vertex(vertex);
                self.vertices[vertex].reindex = copy as i32;
                self.vertices[copy].attributes = self.vertices[vertex].attributes;
                for attribute in [0, 4] {
                    let position = self.vertices[vertex].attributes[attribute];
                    let a = self.vertices[previous].attributes[attribute];
                    let b = self.vertices[next].attributes[attribute];
                    let d0 = unit(std::array::from_fn(|i| a[i] - position[i]));
                    let d1 = unit(std::array::from_fn(|i| b[i] - position[i]));
                    let mut delta = std::array::from_fn::<_, 3, _>(|i| d0[i] + d1[i]);
                    let length =
                        (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                    if length < 1e-5 {
                        let normal = self.vertices[vertex].attributes[1];
                        delta = [
                            d1[1] * normal[2] - d1[2] * normal[1],
                            d1[2] * normal[0] - d1[0] * normal[2],
                            d1[0] * normal[1] - d1[1] * normal[0],
                        ];
                    }
                    for (axis, value) in delta.iter().enumerate() {
                        self.vertices[copy].attributes[attribute][axis] += pull_in * value;
                    }
                }
                vertex = self.vertices[vertex].next as usize;
                if mode != 1 || vertex == first {
                    break;
                }
            }
        }
        self.reindex();
        self.faces_to_vertices();
        let mut matrix = identity();
        for (i, column) in matrix[..3].iter_mut().enumerate() {
            column[i] = elevate;
        }
        self.transform(&matrix, 1, 0, true);
    }

    pub fn displace(&mut self, bitmap: &crate::bitmap::Bitmap, amplitude: [f32; 3]) {
        let scale = [bitmap.width as f32 * 65536., bitmap.height as f32 * 65536.];
        for i in 0..self.vertices.len() {
            if !self.vertices[i].element.selected {
                continue;
            }
            let mut vertex = i;
            let mut count = 0;
            let mut displacement = [0.; 3];
            loop {
                let attributes = self.vertices[vertex].attributes;
                let pixel = bitmap.sample(
                    (attributes[4][0] * scale[0]) as i32,
                    (attributes[4][1] * scale[1]) as i32,
                    0,
                );
                let height = (pixel[0] as f32 - 16384.) * (1. / 32767.);
                for axis in 0..3 {
                    displacement[axis] += attributes[1][axis] * height * amplitude[axis];
                }
                self.vertices[vertex].element.selected = false;
                vertex = self.vertices[vertex].next as usize;
                count += 1;
                if vertex == i {
                    break;
                }
            }
            let inverse = 1. / count as f32;
            for d in &mut displacement {
                *d *= inverse;
            }
            loop {
                for (axis, value) in displacement.iter().enumerate() {
                    self.vertices[vertex].attributes[0][axis] += value;
                }
                vertex = self.vertices[vertex].next as usize;
                if vertex == i {
                    break;
                }
            }
        }
    }
}
