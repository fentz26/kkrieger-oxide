use super::{Collision, Matrix, Mesh, identity, srt_matrix};

/// Original affine MulA: a's columns transformed by b, with affine translation.
pub(super) fn affine(a: Matrix, b: Matrix) -> Matrix {
    let mut result = [[0.; 4]; 4];
    for column in 0..4 {
        for row in 0..3 {
            result[column][row] =
                b[0][row] * a[column][0] + b[1][row] * a[column][1] + b[2][row] * a[column][2];
        }
    }
    for (target, value) in result[3].iter_mut().zip(b[3]).take(3) {
        *target += value;
    }
    result[3][3] = 1.;
    result
}

impl Mesh {
    pub fn bounding_box(&mut self) -> ([f32; 3], [f32; 3]) {
        self.all_to_selection(true, 2);
        self.faces_to_vertices();
        let mut low = [1e15_f32; 3];
        let mut high = [-1e15_f32; 3];
        for vertex in &self.vertices {
            if vertex.element.selected {
                for axis in 0..3 {
                    low[axis] = low[axis].min(vertex.attributes[0][axis]);
                    high[axis] = high[axis].max(vertex.attributes[0][axis]);
                }
            }
        }
        (low, high)
    }

    pub fn auto_collision(
        &mut self,
        enlarge: f32,
        mode: i32,
        tessellation: [usize; 3],
        output_mask: u32,
    ) {
        self.all_to_selection(true, 7);
        let (low, high) = self.bounding_box();
        let mut cells = Self::collision_cube(
            [
                low[0] - enlarge,
                high[0] + enlarge,
                low[1] - enlarge,
                high[1] + enlarge,
                low[2] - enlarge,
                high[2] + enlarge,
            ],
            mode,
            tessellation,
            [0; 4],
            [None; 2],
        );
        self.all_to_mask(output_mask.wrapping_shl(16), super::Selection::SetNot);
        cells.all_to_mask(output_mask.wrapping_shl(16), super::Selection::Set);
        self.append(&cells);
    }

    pub fn center(&mut self, which: u32) {
        let (low, high) = self.bounding_box();
        self.all_to_selection(true, 4);
        let mut matrix = identity();
        if which & 1 != 0 {
            matrix[3][0] = -0.5 * (low[0] + high[0]);
        }
        if which & 2 != 0 {
            matrix[3][1] = if which & 128 != 0 {
                -low[1]
            } else {
                -0.5 * (low[1] + high[1])
            };
        }
        if which & 4 != 0 {
            matrix[3][2] = -0.5 * (low[2] + high[2]);
        }
        self.transform(&matrix, 0, 0, false);
    }

    pub fn multiplied(
        &mut self,
        srt: [f32; 9],
        count: i32,
        mode: u32,
        uv: [f32; 2],
        local_rotation: [f32; 3],
        extrude: f32,
    ) -> Self {
        self.all_to_selection(true, 7);
        let step = srt_matrix(&srt);
        let local_step = srt_matrix(&[
            1.,
            1.,
            1.,
            local_rotation[0],
            local_rotation[1],
            local_rotation[2],
            0.,
            0.,
            0.,
        ]);
        let mut transform = identity();
        let mut local = identity();
        let mut output = Self::default();
        if extrude != 0. {
            self.need_all_normals();
        }
        let mut rng = crate::random::Random::new(count);
        for j in 0..count {
            output.append(self);
            let start = output.vertices.len() - self.vertices.len();
            for (i, v) in output.vertices.iter_mut().enumerate() {
                v.element.selected = i >= start;
            }
            if extrude != 0. {
                let mut matrix = identity();
                for (axis, column) in matrix[..3].iter_mut().enumerate() {
                    column[axis] = j as f32 * extrude;
                }
                output.transform(&matrix, 1, 0, true);
            }
            output.transform(&affine(local, transform), 0, 0, false);
            if mode & 2 != 0 {
                let mut random = [0.; 9];
                for axis in 0..3 {
                    random[axis] = (srt[axis] - 1.) * rng.float() + 1.;
                }
                for axis in 3..9 {
                    random[axis] = srt[axis] * (rng.float() - 0.5) * 2.;
                }
                transform = srt_matrix(&random);
            } else {
                transform = affine(transform, step);
            }
            local = affine(local, local_step);
            if mode & 1 != 0 {
                let mut matrix = identity();
                matrix[3][0] = j as f32 * uv[0];
                matrix[3][1] = j as f32 * uv[1];
                output.transform(&matrix, 4, 4, false);
            }
        }
        output.got_normals = false;
        output
    }

    /// Original GenMesh::Add: preserve element IDs and merge identical material/pass pairs.
    pub fn append(&mut self, other: &Mesh) {
        let vertices = self.vertices.len() as i32;
        let edges = self.edges.len() as i32;
        let faces = self.faces.len() as i32;
        self.vertices
            .extend(other.vertices.iter().cloned().map(|mut v| {
                v.next += vertices;
                v.first += vertices;
                v.reindex += vertices;
                v
            }));
        self.edges
            .extend(other.edges.iter().cloned().map(|mut edge| {
                for side in 0..2 {
                    edge.next[side] += edges * 2;
                    edge.previous[side] += edges * 2;
                    edge.vertex[side] += vertices;
                    edge.face[side] += faces;
                }
                edge
            }));
        let mut remap = vec![0; other.materials.len()];
        for (i, &material) in other.materials.iter().enumerate().skip(1) {
            remap[i] = self
                .materials
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(i, &m)| (m == material).then_some(i))
                .unwrap_or_else(|| {
                    self.materials.push(material);
                    self.materials.len() - 1
                });
        }
        self.faces
            .extend(other.faces.iter().cloned().map(|mut face| {
                face.edge += edges * 2;
                face.material = remap[face.material as usize] as i32;
                face
            }));
        self.collision
            .extend(other.collision.iter().cloned().map(|mut cell| {
                for v in &mut cell.vertices {
                    *v += vertices as usize;
                }
                cell
            }));
        self.light_slots
            .extend(other.light_slots.iter().map(|&v| v + vertices as usize));
        self.parts
            .extend(other.parts.iter().map(|&f| f + faces as usize));
        self.pivot = None;
    }

    /// Original CollisionCube, including cylindrical wedges and event logic.
    pub fn collision_cube(
        bounds: [f32; 6],
        mode: i32,
        tessellation: [usize; 3],
        logic: [i32; 4],
        events: [Option<usize>; 2],
    ) -> Self {
        let [tx, ty, mut tz] = tessellation;
        assert!(tx > 0 && ty > 0 && tz > 0);
        if mode & 16 != 0 {
            tz = tz.max(3);
        }
        let [x0, x1, y0, y1, z0, z1] = bounds;
        let mut scale = [x0 - x1, y0 - y1, z0 - z1];
        let origin = if mode & 16 == 0 {
            scale[0] /= tx as f32;
            scale[1] /= ty as f32;
            scale[2] /= tz as f32;
            [x1, y1, z1]
        } else {
            scale[0] /= -(tx as f32) * 2.;
            scale[1] /= -(ty as f32);
            scale[2] /= -(tx as f32) * 2.;
            [(x0 + x1) * 0.5, y0, (z0 + z1) * 0.5]
        };
        let mut mesh = Self::default();
        for ix in 0..tx {
            for iy in 0..ty {
                for iz in 0..tz {
                    let angles = [
                        (iz + 1) as f32 * std::f32::consts::TAU / tz as f32,
                        iz as f32 * std::f32::consts::TAU / tz as f32,
                    ];
                    let mut vertices = [0; 8];
                    for (i, index) in vertices.iter_mut().enumerate() {
                        *index = mesh.add_vertex();
                        let position = if mode & 16 != 0 {
                            [
                                (f64::from(origin[0])
                                    + libm::sin(f64::from(angles[i & 1]))
                                        * f64::from(scale[0])
                                        * (ix + ((i >> 2) & 1)) as f64)
                                    as f32,
                                origin[1] + scale[1] * (iy + ((i >> 1) & 1)) as f32,
                                (f64::from(origin[2])
                                    + libm::cos(f64::from(angles[i & 1]))
                                        * f64::from(scale[2])
                                        * (ix + ((i >> 2) & 1)) as f64)
                                    as f32,
                                1.,
                            ]
                        } else {
                            [
                                origin[0] + scale[0] * (ix + (i & 1)) as f32,
                                origin[1] + scale[1] * (iy + ((i >> 1) & 1)) as f32,
                                origin[2] + scale[2] * (iz + ((i >> 2) & 1)) as f32,
                                1.,
                            ]
                        };
                        mesh.vertices[*index].attributes[0] = position;
                    }
                    mesh.collision.push(Collision {
                        vertices,
                        mode: mode & 15,
                        code: logic[0] as u8,
                        condition: logic[1] as u8,
                        value: logic[2],
                        output: logic[3] as u8,
                        events,
                    });
                }
            }
        }
        mesh
    }

    pub fn need_all_normals(&mut self) {
        if !self.got_normals {
            self.calculate_normals(false, 7);
            self.got_normals = true;
        }
    }
}
