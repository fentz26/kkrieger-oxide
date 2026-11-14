use super::{Mesh, Selection};

fn attribute_mask(flags: u32) -> u32 {
    let mut output = 0;
    let mut packed = 1;
    for bit in 0..6 {
        if 47 & (1 << bit) != 0 {
            if flags & (1 << bit) != 0 {
                output |= packed;
            }
            packed <<= 1;
        }
    }
    output
}

impl Mesh {
    pub fn reindex(&mut self) {
        let indices: Vec<_> = self.vertices.iter().map(|v| v.reindex).collect();
        for (i, v) in self.vertices.iter_mut().enumerate() {
            if v.reindex == i as i32 {
                v.first = indices[v.first as usize];
                v.next = indices[v.next as usize];
            } else {
                v.first = i as i32;
                v.next = i as i32;
                v.element.mask = 0;
                v.element.selected = false;
            }
        }
        for edge in &mut self.edges {
            for v in &mut edge.vertex {
                *v = indices[*v as usize];
            }
        }
        for cell in &mut self.collision {
            for v in &mut cell.vertices {
                *v = indices[*v] as usize;
            }
        }
        for (i, v) in self.vertices.iter_mut().enumerate() {
            v.reindex = i as i32;
        }
    }

    /// Original selected-face subdivision, including attribute-specific creases.
    pub fn subdivide(&mut self, alpha: f32) {
        let faces = self.faces.len();
        let original_vertices = self.vertices.len();
        for (i, v) in self.vertices.iter_mut().enumerate() {
            v.temp = -1;
            v.temp2 = i as i32;
        }
        for face in 0..faces {
            self.faces[face].temp = -1;
            self.faces[face].temp2 = -1;
            if !self.faces[face].element.selected {
                continue;
            }
            let first = self.faces[face].edge as usize;
            let center = self.add_vertex();
            self.faces[face].temp2 = center as i32;
            self.vertices[center].element.selected = self.faces[face].element.selected;
            self.vertices[center].element.mask = self.faces[face].element.mask;
            let mut edge = first;
            let mut count = 0;
            loop {
                let vertex = self.vertex_id(edge);
                let data = self.vertices[vertex].attributes;
                for (target, value) in self.vertices[center]
                    .attributes
                    .iter_mut()
                    .flatten()
                    .zip(data.iter().flatten())
                {
                    *target += value;
                }
                if self.vertices[vertex].reindex == vertex as i32 {
                    let copy = self.add_copied_vertex(vertex);
                    self.vertices[vertex].temp = edge as i32;
                    self.vertices[vertex].temp2 = vertex as i32;
                    self.vertices[vertex].reindex = copy as i32;
                    self.vertices[copy].temp2 = vertex as i32;
                }
                edge = self.next_face_edge(edge);
                count += 1;
                if edge == first {
                    break;
                }
            }
            let scale = 1. / count as f32;
            for value in self.vertices[center].attributes.iter_mut().flatten() {
                *value *= scale;
            }
        }
        for i in 0..original_vertices {
            if self.vertices[i].reindex == i as i32 || self.vertices[i].temp < 0 {
                continue;
            }
            let first = self.vertices[i].temp as usize;
            let mut edge = first;
            let mut neighbors = Vec::new();
            let mut creases = 0;
            let mut boundary = None;
            loop {
                neighbors.push(self.vertex_id(edge ^ 1));
                let center = self.faces[self.face_id(edge)].temp2;
                if center != -1 {
                    neighbors.push(center as usize);
                } else {
                    boundary = Some(edge);
                }
                if self.vertex_id(edge) == i {
                    creases |= self.edges[edge >> 1].crease;
                }
                edge = self.next_vertex_edge(edge);
                if edge == first {
                    break;
                }
            }
            let destination = self.vertices[i].reindex as usize;
            let original = self.vertices[i].attributes;
            let mut result = [[0.; 4]; 5];
            if let Some(mut edge) = boundary {
                let a = self.vertex_id(edge ^ 1);
                loop {
                    edge = self.next_vertex_edge(edge);
                    if !self.faces[self.face_id(edge ^ 1)].element.selected {
                        break;
                    }
                }
                let b = self.vertex_id(edge ^ 1);
                let weight = alpha * 0.125;
                let retain = 1. - 2. * weight;
                for attribute in 0..5 {
                    for axis in 0..4 {
                        result[attribute][axis] = (self.vertices[a].attributes[attribute][axis]
                            + self.vertices[b].attributes[attribute][axis])
                            * weight;
                        result[attribute][axis] += original[attribute][axis] * retain;
                    }
                }
            } else {
                let count = neighbors.len();
                let weight = 4. * alpha / (count * count) as f32;
                let retain = 1. - alpha * 4. / count as f32;
                let mask = attribute_mask(creases as u32);
                for attribute in 0..5 {
                    if mask & (1 << attribute) != 0 {
                        result[attribute] = original[attribute];
                    } else {
                        for axis in 0..4 {
                            result[attribute][axis] = original[attribute][axis] * retain;
                        }
                        for &neighbor in &neighbors {
                            for (target, source) in result[attribute]
                                .iter_mut()
                                .zip(self.vertices[neighbor].attributes[attribute])
                            {
                                *target += source * weight;
                            }
                        }
                    }
                }
            }
            self.vertices[destination].attributes = result;
        }
        let weight = alpha * 0.25;
        let retain = 0.5 - weight;
        let original_edges = self.edges.len();
        for edge in (0..original_edges * 2).step_by(2) {
            let selected = self.faces[self.face_id(edge)].element.selected;
            let opposite = self.faces[self.face_id(edge ^ 1)].element.selected;
            if !selected && !opposite {
                continue;
            }
            let vertex = self.add_vertex();
            self.vertices[vertex].element.mask = self.vertices[self.vertex_id(edge)].element.mask
                | self.vertices[self.vertex_id(self.next_face_edge(self.next_face_edge(edge)))]
                    .element
                    .mask;
            let (a, b, _) = self.split_bridge(
                self.next_vertex_edge(edge ^ 1),
                self.previous_vertex_edge(edge ^ 1),
                vertex,
                0x100000,
                Selection::Add,
            );
            let b0 =
                self.vertices[self.vertex_id(self.previous_face_edge(edge ^ 1))].temp2 as usize;
            let b1 = self.vertices[self.vertex_id(self.next_face_edge(edge ^ 1))].temp2 as usize;
            let a0 = self.vertices[self.vertex_id(edge)].temp2 as usize;
            let a1 = self.vertices[self.vertex_id(self.next_face_edge(self.next_face_edge(edge)))]
                .temp2 as usize;
            let crease = self.edges[edge >> 1].crease;
            let mask = if selected && opposite {
                attribute_mask(!crease as u32)
            } else {
                0
            };
            let center0 = self.faces[self.face_id(edge)].temp2 as usize;
            let center1 = self.faces[self.face_id(edge ^ 1)].temp2 as usize;
            for attribute in 0..5 {
                let t0 = std::array::from_fn::<_, 4, _>(|axis| {
                    self.vertices[a0].attributes[attribute][axis]
                        + self.vertices[a1].attributes[attribute][axis]
                });
                let t1 = if a != b {
                    std::array::from_fn(|axis| {
                        self.vertices[b0].attributes[attribute][axis]
                            + self.vertices[b1].attributes[attribute][axis]
                    })
                } else {
                    [0.; 4]
                };
                if mask & (1 << attribute) != 0 {
                    self.vertices[a].attributes[attribute] = t0.map(|v| v * retain);
                    if a != b {
                        self.vertices[b].attributes[attribute] = t1.map(|v| v * retain);
                    }
                    let centers = std::array::from_fn::<_, 4, _>(|axis| {
                        self.vertices[center0].attributes[attribute][axis]
                            + self.vertices[center1].attributes[attribute][axis]
                    });
                    for (axis, value) in centers.iter().enumerate() {
                        self.vertices[a].attributes[attribute][axis] += value * weight;
                        if a != b {
                            self.vertices[b].attributes[attribute][axis] += value * weight;
                        }
                    }
                } else {
                    self.vertices[a].attributes[attribute] = t0.map(|v| v * 0.5);
                    if a != b {
                        self.vertices[b].attributes[attribute] = t1.map(|v| v * 0.5);
                    }
                }
            }
            if selected {
                let face = self.face_id(edge);
                self.faces[face].temp = edge as i32;
            }
            if opposite {
                let face = self.face_id(edge ^ 1);
                self.faces[face].temp = self.previous_face_edge(edge ^ 1) as i32;
            }
        }
        let count = self.faces.len();
        for face in 0..count {
            if !self.faces[face].element.selected {
                continue;
            }
            let first = self.faces[face].temp as usize;
            let mut edge = first;
            let mut boundaries = Vec::new();
            loop {
                edge = self.next_face_edge(edge);
                boundaries.push(edge);
                edge = self.next_face_edge(edge);
                if edge == first {
                    break;
                }
            }
            let vertex = self.faces[face].temp2 as usize;
            self.split_bridge(
                self.next_vertex_edge(boundaries[0]),
                boundaries[0],
                vertex,
                0,
                Selection::Add,
            );
            for &edge in &boundaries[1..] {
                self.split_face(
                    self.previous_face_edge(self.previous_face_edge(self.previous_face_edge(edge))),
                    edge,
                    0,
                    Selection::Add,
                );
            }
        }
        self.reindex();
    }
}
