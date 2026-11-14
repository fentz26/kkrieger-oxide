use super::{Mesh, Selection, euler_matrix, identity};

impl Mesh {
    pub fn skip_face_edge(&self, mut edge: usize, offset: i32) -> usize {
        for _ in 0..offset.unsigned_abs() {
            edge = if offset < 0 {
                self.previous_face_edge(edge)
            } else {
                self.next_face_edge(edge)
            };
        }
        edge
    }

    fn multi_split(&mut self, mut edge: usize, splits: usize) {
        let a0 = self.vertex_id(edge);
        let a1 = self.vertex_id(self.next_face_edge(edge));
        let b0 = self.vertex_id(self.next_face_edge(edge ^ 1));
        let b1 = self.vertex_id(edge ^ 1);
        for i in 0..splits {
            let vertex = self.add_vertex();
            let (a, b, _) = self.split_bridge(
                self.next_vertex_edge(edge ^ 1),
                self.previous_vertex_edge(edge ^ 1),
                vertex,
                0,
                Selection::Add,
            );
            let t = (i as f32 + 1.) / (splits as f32 + 1.);
            for attribute in 0..5 {
                self.vertices[a].attributes[attribute] = std::array::from_fn(|axis| {
                    let start = self.vertices[a0].attributes[attribute][axis];
                    start + (self.vertices[a1].attributes[attribute][axis] - start) * t
                });
                self.vertices[b].attributes[attribute] = std::array::from_fn(|axis| {
                    let start = self.vertices[b0].attributes[attribute][axis];
                    start + (self.vertices[b1].attributes[attribute][axis] - start) * t
                });
            }
            edge = self.next_face_edge(edge);
            self.edges[edge >> 1].element.selected = true;
        }
    }

    fn quad_split(&mut self, mut edge: usize, splits: usize, offset: i32, target: i32) {
        if splits == 0 {
            return;
        }
        let a = self.skip_face_edge(edge, offset);
        let b = self.skip_face_edge(a, 2);
        for edge in [a, b] {
            if !self.edges[edge >> 1].element.selected {
                self.edges[edge >> 1].element.selected = true;
                self.multi_split(edge, splits);
            }
        }
        for _ in 0..splits {
            let split = self.skip_face_edge(edge, target + 1);
            self.split_face(split, self.skip_face_edge(split, -3), 0, Selection::Add);
            edge = self.skip_face_edge(split, -target);
        }
    }

    pub fn select_logic(&mut self, first: u32, second: u32, destination: u32, mode: u32) {
        self.all_to_selection(true, 7);
        let source = first | second;
        if source & 255 != 0 && destination & 255 != 0 {
            for e in &mut self.edges {
                e.element.logic(first, second, destination, mode & 31);
            }
        }
        if source & 0xff00 != 0 && destination & 0xff00 != 0 {
            for f in &mut self.faces {
                f.element
                    .logic(first >> 8, second >> 8, destination >> 8, mode & 31);
            }
        }
        if source & 0xff0000 != 0 && destination & 0xff0000 != 0 {
            for v in &mut self.vertices {
                v.element
                    .logic(first >> 16, second >> 16, destination >> 16, mode & 31);
            }
        }
        if destination & 0xff0000 != 0 && destination & 0xff00 != 0 && mode & 32 != 0 {
            self.mask_to_selection(destination & 0xff00);
            self.faces_to_vertices();
            self.selection_to_mask(destination & 0xff0000, Selection::Set);
        }
    }

    pub fn grid(mode: u32, tes_u: usize, tes_v: usize) -> Self {
        assert!(tes_u > 0 && tes_v > 0);
        let mut mesh = Self::ring(
            4,
            0,
            std::f32::consts::SQRT_2 / 2.,
            std::f32::consts::TAU / 8.,
            0,
        );
        mesh.all_to_selection(true, 7);
        mesh.faces[1].element.selected = false;
        mesh.crease(0, 0, Selection::Add, 254);
        mesh.faces[1].element.selected = true;
        let mut matrix = identity();
        matrix[1][1] = -1.;
        matrix[3][0] = 0.5;
        matrix[3][1] = 0.5;
        mesh.transform(&matrix, 0, 4, false);
        mesh.transform(
            &euler_matrix([0.25 * std::f32::consts::TAU, 0., 0.]),
            0,
            0,
            false,
        );
        mesh.calculate_normals(false, 3);
        for v in &mut mesh.vertices {
            if v.attributes[1][1] < 0. {
                v.attributes[4][0] = 1. - v.attributes[4][0];
            }
        }
        mesh.all_to_selection(false, 7);
        mesh.quad_split(mesh.faces[0].edge as usize, tes_v - 1, 0, 0);
        mesh.quad_split(mesh.faces[1].edge as usize, tes_v - 1, 0, 2);
        mesh.all_to_selection(false, 7);
        for face in 0..tes_v * 2 {
            mesh.quad_split(mesh.faces[face].edge as usize, tes_u - 1, -1, 1);
        }
        mesh.all_to_mask(u32::MAX, Selection::SetNot);
        mesh.all_to_selection(false, 7);
        for face in 0..mesh.faces.len() {
            mesh.faces[face].element.selected =
                mesh.vertices[mesh.vertex_id(mesh.faces[face].edge as usize)].attributes[1][1] < 0.;
        }
        mesh.faces_to_vertices();
        mesh.selection_to_mask(0x10100, Selection::Set);
        mesh.select_logic(0x10100, 0x10100, 0x20200, 19);
        for axis in [0, 2] {
            mesh.all_to_selection(false, 7);
            for edge in 0..mesh.edges.len() * 2 {
                if mesh.edges[edge >> 1].crease != 0 {
                    let a = mesh.vertices[mesh.vertex_id(edge)].attributes[0][axis];
                    let b = mesh.vertices[mesh.vertex_id(edge ^ 1)].attributes[0][axis];
                    if (a - b).abs() > 1e-6 {
                        let face = mesh.face_id(edge);
                        mesh.faces[face].element.selected = true;
                    }
                }
            }
            mesh.faces_to_vertices();
            mesh.selection_to_mask(0x40400 << axis, Selection::Set);
            mesh.select_logic(0x40400 << axis, 0x40400 << axis, 0x80800 << axis, 19);
        }
        mesh.select_logic(0x40400, 0x101000, 0x404000, 0);
        mesh.select_logic(0x404000, 0x404000, 0x808000, 19);
        if mode & 1 == 0 {
            mesh.mask_to_selection(0x200);
            mesh.delete_selected_faces();
        }
        mesh.parts.push(0);
        mesh
    }
}
