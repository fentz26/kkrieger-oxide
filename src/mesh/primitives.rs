use super::{Mesh, Selection, identity, srt_matrix};

impl Mesh {
    pub fn triangulate(&mut self, threshold: usize, mask: u32, selection: Selection, kind: i32) {
        let mut face = 0;
        while face < self.faces.len() {
            if self.faces[face].element.selected {
                let first = self.faces[face].edge as usize;
                let mut edges = Vec::new();
                let mut vertices = Vec::new();
                let mut edge = first;
                loop {
                    edges.push(edge);
                    vertices.push(self.vertex_id(edge));
                    edge = self.next_face_edge(edge);
                    if edge == first {
                        break;
                    }
                }
                let count = edges.len();
                if count >= threshold {
                    match kind {
                        0 => {
                            let center = self.add_vertex();
                            self.vertices[center]
                                .element
                                .select(mask >> 16, true, selection);
                            let weight = 1. / count as f32;
                            for j in 0..count {
                                let source = self.vertices[vertices[j]].attributes;
                                for (target, source) in self.vertices[center]
                                    .attributes
                                    .iter_mut()
                                    .flatten()
                                    .zip(source.iter().flatten())
                                {
                                    *target += source * weight;
                                }
                                if j == 0 {
                                    self.split_bridge(
                                        self.next_vertex_edge(edges[0]),
                                        edges[0],
                                        center,
                                        mask,
                                        selection,
                                    );
                                } else {
                                    self.split_face(
                                        self.previous_face_edge(self.previous_face_edge(edges[j])),
                                        edges[j],
                                        mask,
                                        selection,
                                    );
                                }
                            }
                        }
                        1 => {
                            for &edge in &edges[..count.saturating_sub(3)] {
                                self.split_face(
                                    self.previous_face_edge(self.previous_face_edge(edge)),
                                    edge,
                                    mask,
                                    selection,
                                );
                            }
                        }
                        2 => {
                            let mut start = 0;
                            for j in 0..count {
                                if self.vertices[vertices[j]].attributes[0][2]
                                    < self.vertices[vertices[start]].attributes[0][2]
                                {
                                    start = j;
                                }
                            }
                            let range = if start != 0 {
                                start + 3..start + count
                            } else {
                                start + 1..(start + count).saturating_sub(2)
                            };
                            for j in range {
                                let edge = edges[j % count];
                                self.split_face(
                                    self.previous_face_edge(self.previous_face_edge(edge)),
                                    edge,
                                    mask,
                                    selection,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            face += 1;
        }
    }

    pub fn selected_edges_to_vertices(&mut self, uv_axis: usize) {
        assert!(uv_axis < 4);
        for half in 0..self.edges.len() * 2 {
            if self.edges[half >> 1].element.selected
                && self.vertices[self.vertex_id(self.previous_face_edge(half))].attributes[4]
                    [uv_axis]
                    > 0.5
            {
                let first = self.vertex_id(half);
                let second = self.vertex_id(self.next_face_edge(half));
                self.vertices[first].element.selected = true;
                self.vertices[second].element.selected = true;
            }
        }
    }

    pub fn delete_selected_faces(&mut self) {
        for face in &mut self.faces {
            if face.element.selected {
                face.material = 0;
                face.element.selected = false;
                face.element.mask = 0;
            }
        }
    }

    pub fn cylinder(tx: usize, ty: usize, mode: u32, tz: usize, arc: usize) -> Self {
        assert!(tx >= 3 && ty > 0 && tz > 0);
        let arc = arc.min(tx - 1);
        let mut mesh = Self::ring(tx, 0, 0.5, 0., arc);
        let mut mat = identity();
        mat[3][2] = 1.;
        let mut uv = identity();
        uv[3][1] = 1. / ty as f32;
        mesh.faces[0].element.selected = true;
        for _ in 0..ty {
            let edge = mesh.edges.len();
            mesh.extrude(0, Selection::Add, 0);
            mesh.edges[edge].element.select(1, true, Selection::Add);
            mesh.faces_to_vertices();
            mesh.transform(&mat, 0, 0, false);
            mesh.transform(&uv, 4, 4, false);
        }
        mat = srt_matrix(&[1., 1., 1., 0.25, 0., 0., 0., 0., 0.]);
        let scale = [1. / tz as f32, 1. / ty as f32, 1. / tz as f32, 1.];
        for column in &mut mat[..3] {
            for (value, scale) in column.iter_mut().zip(scale) {
                *value *= scale;
            }
        }
        mat[3] = [0., 0.5, 0., 1.];
        mesh.all_to_selection(true, 4);
        mesh.transform(&mat, 0, 0, false);
        mesh.all_to_mask(0x10100, Selection::Subtract);
        mesh.select_cube(
            [0., 0.5, 0.],
            [1025., 0.0001, 1025.],
            0x10100,
            Selection::Add,
        );
        mesh.select_cube(
            [0., -0.5, 0.],
            [1025., 0.0001, 1025.],
            0x10100,
            Selection::Add,
        );
        mesh.mask_to_selection(0x100);
        mesh.triangulate(4, 0, Selection::Add, if arc > 0 { 2 } else { 0 });
        if tz > 1 {
            mesh.all_to_selection(true, 4);
            mat = identity();
            mat[1][1] = 0.;
            mesh.transform(&mat, 0, 1, false);
            mesh.mask_to_selection(0x100);
            for face in &mut mesh.faces {
                face.element.selected ^= true;
            }
            for _ in 1..tz {
                let edge = mesh.edges.len();
                mesh.extrude(0x100, Selection::Add, 0);
                for edge in &mut mesh.edges[edge..] {
                    edge.element.mask = 0;
                }
                mesh.faces_to_vertices();
                mesh.transform(&identity(), 1, 0, true);
            }
        }
        for sign in [-1., 1.] {
            mesh.all_to_selection(false, 6);
            mesh.select_cube([0., sign * 0.5, 0.], [2., 0.01, 2.], 0, Selection::Add);
            mesh.crease(0, 0, Selection::Add, 254);
            mesh.faces_to_vertices();
            uv = identity();
            uv[1][1] = 0.;
            uv[2][1] = -sign;
            uv[3] = [0.5, 0.5, 0., 1.];
            mesh.transform(&uv, 0, 4, false);
        }
        mesh.all_to_mask(0x20000, Selection::Subtract);
        mesh.mask_to_selection(0x20001);
        mesh.crease(1, 0, Selection::Add, 32);
        mesh.selected_edges_to_vertices(0);
        uv = identity();
        uv[3][0] = 1.;
        mesh.transform(&uv, 4, 4, false);
        if mode & 1 != 0 {
            mesh.mask_to_selection(0x100);
            mesh.delete_selected_faces();
        }
        if mode & 2 != 0 {
            mat = identity();
            mat[3][1] = 0.5;
            mesh.all_to_selection(true, 7);
            mesh.transform(&mat, 0, 0, false);
        }
        mesh.all_to_mask(1, Selection::Subtract);
        mesh.parts.push(0);
        mesh
    }
}
