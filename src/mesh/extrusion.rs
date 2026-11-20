use super::{Matrix, Mesh, Selection, srt_matrix};

fn rotate(vector: [f32; 3], matrix: &Matrix) -> [f32; 3] {
    std::array::from_fn(|axis| {
        vector[0] * matrix[0][axis] + vector[1] * matrix[1][axis] + vector[2] * matrix[2][axis]
    })
}
fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if length > 1e-20 {
        let inverse = 1. / length.sqrt();
        v.map(|v| v * inverse)
    } else {
        [1., 0., 0.]
    }
}

impl Mesh {
    /// Mesh_Extrude's connected-face groups, normal modes and local scaling.
    /// Input selection follows the caller's CheckMesh source mask.
    pub fn extrude_faces(
        &mut self,
        destination: u32,
        mode: u32,
        count: usize,
        mut distance: [f32; 3],
        mut scale: [f32; 3],
        mut rotation: [f32; 3],
    ) {
        self.need_all_normals();
        let mut groups = if mode & 8 != 0 || mode & 6 == 2 { 0 } else { 1 };
        for face in &mut self.faces {
            face.temp = groups - 1;
        }
        while let Some(first) = self
            .faces
            .iter()
            .position(|f| f.temp == -1 && f.element.selected)
        {
            let mut pending = vec![first];
            while let Some(face) = pending.pop() {
                self.faces[face].temp = groups;
                let first = self.faces[face].edge as usize;
                let mut edge = first;
                loop {
                    let opposite = self.face_id(edge ^ 1);
                    if self.faces[opposite].temp == -1
                        && !self.is_border_edge(edge, (mode & 1) as i32)
                    {
                        self.faces[opposite].temp = -2;
                        pending.push(opposite);
                    }
                    edge = self.next_face_edge(edge);
                    if edge == first {
                        break;
                    }
                }
            }
            groups += 1;
        }
        let inverse = 1. / count as f32;
        for axis in 0..3 {
            scale[axis] = (libm::pow(f64::from(scale[axis].abs()), f64::from(inverse))
                * if scale[axis] < 0. { -1. } else { 1. }) as f32;
            rotation[axis] *= inverse;
            distance[axis] *= inverse;
        }
        let matrix = srt_matrix(&[
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
        let selection = match (mode >> 4) & 3 {
            0 => Selection::Add,
            1 => Selection::Subtract,
            2 => Selection::Set,
            _ => Selection::SetNot,
        };
        if (mode >> 4) & 3 >= 2 {
            self.all_to_mask((destination & 255) << 8, Selection::Subtract);
        }
        let mut normal = if mode & 6 == 4 { distance } else { [0.; 3] };
        for _ in 0..count {
            self.extrude((destination & 255) << 8, selection, (mode & 1) as i32);
            for vertex in &mut self.vertices {
                vertex.temp = 0;
            }
            for group in 0..groups {
                let mut first = None;
                for face in (0..self.faces.len()).rev() {
                    if self.faces[face].element.selected && self.faces[face].temp == group {
                        self.faces[face].temp2 = first.map_or(-1, |i| i as i32);
                        first = Some(face);
                    }
                }
                let mut faces = Vec::new();
                let mut next = first;
                while let Some(face) = next {
                    faces.push(face);
                    next = usize::try_from(self.faces[face].temp2).ok();
                }
                if mode & 6 == 6 {
                    for &face in &faces {
                        let first = self.faces[face].edge as usize;
                        let normal = unit(self.face_normal(first));
                        let mut edge = first;
                        loop {
                            let vertex = self.vertex_id(edge);
                            self.vertices[vertex].attributes[1] =
                                [normal[0], normal[1], normal[2], 0.];
                            edge = self.next_face_edge(edge);
                            if edge == first {
                                break;
                            }
                        }
                    }
                }
                let mut center = [0.; 3];
                let mut vertex_count = 0;
                for pass in 1..4 {
                    let pass_mask = 1 << pass;
                    match pass {
                        1 => {
                            if mode & 6 != 2 {
                                continue;
                            }
                            normal = [0.; 3];
                        }
                        2 => {
                            if mode & 6 == 2 {
                                normal = unit(normal);
                                for axis in 0..3 {
                                    normal[axis] *= distance[axis];
                                }
                            }
                            center = [0.; 3];
                        }
                        3 => {
                            if mode & 8 == 0 {
                                continue;
                            }
                            let inverse = 1. / vertex_count as f32;
                            for v in &mut center {
                                *v *= inverse;
                            }
                        }
                        _ => unreachable!(),
                    }
                    vertex_count = 0;
                    for &face in &faces {
                        let first = self.faces[face].edge as usize;
                        let mut edge = first;
                        loop {
                            let vertex = self.vertex_id(edge);
                            if self.vertices[vertex].temp & pass_mask == 0 {
                                self.vertices[vertex].temp |= pass_mask;
                                let attributes = &mut self.vertices[vertex].attributes;
                                match pass {
                                    1 => {
                                        for (axis, n) in normal.iter_mut().enumerate() {
                                            *n += attributes[1][axis];
                                        }
                                    }
                                    2 => {
                                        for axis in 0..3 {
                                            attributes[0][axis] += if mode & 6 == 0 || mode & 6 == 6
                                            {
                                                attributes[1][axis] * distance[axis]
                                            } else {
                                                normal[axis]
                                            };
                                        }
                                        if mode & 8 != 0 {
                                            for axis in 0..3 {
                                                center[axis] += attributes[0][axis];
                                            }
                                        } else {
                                            let value = std::array::from_fn(|axis| {
                                                attributes[0][axis] * scale[axis]
                                            });
                                            attributes[0][..3]
                                                .copy_from_slice(&rotate(value, &matrix));
                                        }
                                        let value =
                                            [attributes[1][0], attributes[1][1], attributes[1][2]];
                                        attributes[1][..3].copy_from_slice(&rotate(value, &matrix));
                                    }
                                    3 => {
                                        let value = std::array::from_fn(|axis| {
                                            (attributes[0][axis] - center[axis]) * scale[axis]
                                        });
                                        let result = rotate(value, &matrix);
                                        for axis in 0..3 {
                                            attributes[0][axis] = result[axis] + center[axis];
                                        }
                                        attributes[0][3] = 1.;
                                    }
                                    _ => unreachable!(),
                                }
                                vertex_count += 1;
                            }
                            edge = self.next_face_edge(edge);
                            if edge == first {
                                break;
                            }
                        }
                    }
                }
            }
        }
        if destination & 255 != 0 {
            self.mask_to_selection((destination & 255) << 8);
        }
        self.faces_to_vertices();
        self.selection_to_mask((destination & 0xff00) << 8, Selection::Add);
        self.got_normals = false;
    }
}
