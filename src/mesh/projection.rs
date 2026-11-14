use super::{Matrix, Mesh, Selection, assembly::affine, srt_matrix};

impl Mesh {
    pub fn face_normal(&self, half: usize) -> [f32; 3] {
        let a = self.vertices[self.vertex_id(self.previous_face_edge(half))].attributes[0];
        let b = self.vertices[self.vertex_id(half)].attributes[0];
        let c = self.vertices[self.vertex_id(self.next_face_edge(half))].attributes[0];
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    pub fn cubic_projection(&mut self, matrix: Matrix, mask: u32, real: bool) {
        for side in 0..6 {
            let sign = if real && side & 1 == 0 { -1. } else { 1. };
            self.all_to_selection(false, 6);
            for face in 0..self.faces.len() {
                let selected = if mask == 0 || self.faces[face].element.mask as u32 & mask != 0 {
                    let normal = self.face_normal(self.faces[face].edge as usize);
                    let mut maximum = normal[0];
                    let mut axis = 0;
                    if normal[1].abs() > maximum.abs() {
                        maximum = normal[1];
                        axis = 2;
                    }
                    if normal[2].abs() > maximum.abs() {
                        maximum = normal[2];
                        axis = 4;
                    }
                    if maximum > 0. {
                        axis += 1;
                    }
                    axis == side
                } else {
                    false
                };
                self.faces[face].element.selected = selected;
            }
            self.crease(0, 0, Selection::Add, 32);
            self.faces_to_vertices();
            let mut projection = [[0.; 4]; 4];
            match side / 2 {
                0 => {
                    projection[1][1] = -1.;
                    projection[2][0] = sign;
                }
                1 => {
                    projection[0][0] = 1.;
                    projection[2][1] = -sign;
                }
                _ => {
                    projection[0][0] = -sign;
                    projection[1][1] = -1.;
                }
            }
            self.transform(&affine(matrix, projection), 0, 4, false);
        }
    }

    pub fn uv_projection(&mut self, mask: u32, srt: [f32; 9], kind: i32) {
        self.all_to_selection(true, 7);
        let matrix = srt_matrix(&srt);
        if kind == 0 || kind == 3 {
            self.cubic_projection(matrix, mask, kind == 0);
        } else {
            for vertex in &mut self.vertices {
                let position = vertex.attributes[0];
                let point = std::array::from_fn::<_, 3, _>(|axis| {
                    position[0] * matrix[0][axis]
                        + position[1] * matrix[1][axis]
                        + position[2] * matrix[2][axis]
                        + matrix[3][axis]
                });
                let horizontal = point[0] * point[0] + point[2] * point[2];
                vertex.attributes[4][0] = 0.5 - point[0].atan2(point[2]) / std::f32::consts::TAU;
                vertex.attributes[4][1] = if kind == 1 {
                    -point[1]
                } else {
                    0.5 - point[1].atan2(horizontal.sqrt()) / std::f32::consts::PI
                };
            }
        }
    }
}
