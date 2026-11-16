//! Original GenMesh half-edge storage. Half-edge low bits select edge direction.
//!
//! Derived from Farbrausch genmesh.cpp; see third_party/farbrausch-LICENSE.txt.
mod assembly;
mod deform;
mod extrusion;
mod grid;
mod lighting;
mod primitives;
mod projection;
mod subdivision;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Add,
    Subtract,
    Set,
    SetNot,
    /// Original SelElem ignores mode values outside 0..=3.
    Unchanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Element {
    pub mask: u8,
    pub id: u8,
    pub selected: bool,
    pub used: bool,
}

impl Default for Element {
    fn default() -> Self {
        Self {
            mask: 0,
            id: 0,
            selected: false,
            used: true,
        }
    }
}

impl Element {
    pub fn select(&mut self, mask: u32, state: bool, mode: Selection) {
        let mask = mask as u8;
        match mode {
            Selection::Add if state => self.mask |= mask,
            Selection::Subtract if state => self.mask &= !mask,
            Selection::Set => {
                if state {
                    self.mask |= mask
                } else {
                    self.mask &= !mask
                }
            }
            Selection::SetNot => {
                if state {
                    self.mask &= !mask
                } else {
                    self.mask |= mask
                }
            }
            _ => {}
        }
    }

    pub fn logic(&mut self, first: u32, second: u32, destination: u32, mode: u32) {
        let mut a = (self.mask as u32 & first != 0) ^ (mode & 4 != 0);
        let b = (self.mask as u32 & second != 0) ^ (mode & 8 != 0);
        a = match mode & 3 {
            0 => a | b,
            1 => a & b,
            2 => a ^ b,
            _ => a,
        };
        a ^= mode & 16 != 0;
        self.select(destination, a, Selection::Set);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub element: Element,
    pub next: i32,
    pub first: i32,
    pub temp: i32,
    pub temp2: i32,
    pub reindex: i32,
    pub weight: [u8; 4],
    pub matrix: [u8; 4],
    /// Position, normal, tangent, color, UV: original sGMF_DEFAULT layout.
    pub attributes: [[f32; 4]; 5],
}

impl Vertex {
    fn new(index: i32) -> Self {
        Self {
            element: Element::default(),
            next: index,
            first: index,
            temp: -1,
            temp2: -1,
            reindex: index,
            weight: [0; 4],
            matrix: [255; 4],
            attributes: [[0.; 4]; 5],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub element: Element,
    pub next: [i32; 2],
    pub previous: [i32; 2],
    pub face: [i32; 2],
    pub vertex: [i32; 2],
    pub temp: [i32; 2],
    pub crease: i32,
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            element: Element::default(),
            next: [-1; 2],
            previous: [-1; 2],
            face: [-1; 2],
            vertex: [-1; 2],
            temp: [-1; 2],
            crease: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub element: Element,
    pub material: i32,
    pub edge: i32,
    pub temp: i32,
    pub temp2: i32,
    pub temp3: i32,
}

impl Default for Face {
    fn default() -> Self {
        Self {
            element: Element::default(),
            material: 1,
            edge: -1,
            temp: -1,
            temp2: 0,
            temp3: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub faces: Vec<Face>,
    pub collision: Vec<Collision>,
    pub parts: Vec<usize>,
    pub materials: Vec<Material>,
    pub light_slots: Vec<usize>,
    pub pivot: Option<usize>,
    pub got_normals: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialSource {
    Null,
    Default,
    Operator(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Material {
    pub source: MaterialSource,
    pub pass: i32,
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
            collision: Vec::new(),
            parts: Vec::new(),
            materials: vec![
                Material {
                    source: MaterialSource::Null,
                    pass: 0,
                },
                Material {
                    source: MaterialSource::Default,
                    pass: 0,
                },
            ],
            light_slots: Vec::new(),
            pivot: None,
            got_normals: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collision {
    pub vertices: [usize; 8],
    pub mode: i32,
    pub code: u8,
    pub condition: u8,
    pub value: i32,
    pub output: u8,
    pub events: [Option<usize>; 2],
}

pub type Matrix = [[f32; 4]; 4];

pub fn identity() -> Matrix {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ]
}

/// Original InitSRT: scales, Euler rotations in turns, translation.
pub fn srt_matrix(srt: &[f32; 9]) -> Matrix {
    let mut m = identity();
    if srt[3..6].iter().any(|a| a.abs() >= 1e-6) {
        let (sx, cx) = (srt[3] * std::f32::consts::TAU).sin_cos();
        let (sy, cy) = (srt[4] * std::f32::consts::TAU).sin_cos();
        let (sz, cz) = (srt[5] * std::f32::consts::TAU).sin_cos();
        m[0] = [cy * cz, cy * sz, -sy, 0.];
        m[1] = [-cx * sz + sx * sy * cz, cx * cz + sx * sy * sz, sx * cy, 0.];
        m[2] = [sx * sz + cx * sy * cz, -sx * cz + cx * sy * sz, cx * cy, 0.];
    }
    for (column, scale) in m[..3].iter_mut().zip(srt) {
        for x in &mut column[..3] {
            *x *= scale;
        }
    }
    m[3] = [srt[6], srt[7], srt[8], 1.];
    m
}

// Original fsincos evaluates before storing its two single-precision results.
pub fn euler_matrix_fsincos(angles: [f32; 3]) -> Matrix {
    let pairs = angles.map(|a| {
        let (s, c) = f64::from(a).sin_cos();
        (s as f32, c as f32)
    });
    let [(sx, cx), (sy, cy), (sz, cz)] = pairs;
    [
        [cy * cz, cy * sz, -sy, 0.],
        [-cx * sz + sx * sy * cz, cx * cz + sx * sy * sz, sx * cy, 0.],
        [sx * sz + cx * sy * cz, -sx * cz + cx * sy * sz, cx * cy, 0.],
        [0., 0., 0., 1.],
    ]
}

pub fn euler_matrix(angles: [f32; 3]) -> Matrix {
    let (sx, cx) = angles[0].sin_cos();
    let (sy, cy) = angles[1].sin_cos();
    let (sz, cz) = angles[2].sin_cos();
    [
        [cy * cz, cy * sz, -sy, 0.],
        [-cx * sz + sx * sy * cz, cx * cz + sx * sy * sz, sx * cy, 0.],
        [sx * sz + cx * sy * cz, -sx * cz + cx * sy * sz, cx * cy, 0.],
        [0., 0., 0., 1.],
    ]
}

impl Mesh {
    /// Original area-weighted normals and UV-derived tangents, split at creases.
    pub fn calculate_normals(&mut self, smooth: bool, what: u32) {
        fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 3] {
            [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
        }
        fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
            a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
        }
        fn unit(v: [f32; 3]) -> [f32; 3] {
            let length = dot(v, v);
            if length > 1e-20 {
                let scale = 1. / length.sqrt();
                v.map(|v| v * scale)
            } else {
                [1., 0., 0.]
            }
        }
        for v in &mut self.vertices {
            v.temp = -1;
        }
        for half in 0..self.edges.len() * 2 {
            let vertex = self.vertex_id(half);
            self.vertices[vertex].temp = half as i32;
        }
        for component in 0..2 {
            if what & (1 << component) == 0 {
                continue;
            }
            let mask = if smooth {
                0
            } else if component == 0 {
                2
            } else {
                32
            };
            for i in 0..self.vertices.len() {
                if self.vertices[i].temp < 0 {
                    continue;
                }
                let mut edge = self.vertices[i].temp as usize;
                let mut end = edge;
                loop {
                    if self.edges[edge >> 1].crease & mask != 0 {
                        end = edge;
                    } else {
                        edge = self.previous_vertex_edge(edge);
                    }
                    if edge == end {
                        break;
                    }
                }
                let mut write = what & 4 != 0 || self.faces[self.face_id(edge)].element.selected;
                let origin = self.vertices[self.vertex_id(edge)].attributes;
                let mut second =
                    self.vertices[self.vertex_id(self.next_face_edge(edge))].attributes;
                let mut t2 = sub(second[0], origin[0]);
                let mut length2 = dot(t2, t2);
                let mut accumulator = [0.; 3];
                let mut exit = false;
                loop {
                    let was_exit = exit;
                    write |= self.faces[self.face_id(edge)].element.selected;
                    let t1 = t2;
                    let length1 = length2;
                    let first = second;
                    let next = if !was_exit {
                        self.next_face_edge(edge)
                    } else {
                        self.previous_face_edge(self.previous_vertex_edge(edge))
                    };
                    second = self.vertices[self.vertex_id(next)].attributes;
                    t2 = sub(second[0], origin[0]);
                    length2 = dot(t2, t2);
                    if component == 0 {
                        if length1 * length2 > 1e-20 {
                            let t = [
                                t1[1] * t2[2] - t1[2] * t2[1],
                                t1[2] * t2[0] - t1[0] * t2[2],
                                t1[0] * t2[1] - t1[1] * t2[0],
                            ];
                            let scale = 1. / (length1 * length2);
                            for axis in 0..3 {
                                accumulator[axis] += t[axis] * scale;
                            }
                        }
                    } else {
                        let c1 = origin[4][1] - first[4][1];
                        let c2 = second[4][1] - origin[4][1];
                        let ix =
                            (first[4][0] - origin[4][0]) * c2 + (second[4][0] - origin[4][0]) * c1;
                        if ix.abs() > 1e-20 {
                            let inverse = 1. / ix;
                            let mut tangent = t1.map(|v| v * (c2 * inverse));
                            for axis in 0..3 {
                                tangent[axis] += t2[axis] * (c1 * inverse);
                            }
                            if dot(tangent, tangent) > 1e-20 {
                                let tangent = unit(tangent);
                                for axis in 0..3 {
                                    accumulator[axis] += tangent[axis];
                                }
                            }
                        }
                    }
                    edge = self.next_vertex_edge(edge);
                    exit = edge == end || self.edges[edge >> 1].crease & mask != 0;
                    if was_exit {
                        break;
                    }
                }
                if write {
                    if component == 1 {
                        let normal = self.vertices[i].attributes[1];
                        let normal = [normal[0], normal[1], normal[2]];
                        let scale = -dot(accumulator, normal);
                        for axis in 0..3 {
                            accumulator[axis] += normal[axis] * scale;
                        }
                    }
                    let n = unit(accumulator);
                    self.vertices[i].attributes[component + 1] = [n[0], n[1], n[2], 0.];
                }
            }
        }
    }

    pub fn all_to_selection(&mut self, selected: bool, types: u32) {
        if types & 4 != 0 {
            for v in &mut self.vertices {
                v.element.selected = selected;
            }
        }
        if types & 2 != 0 {
            for f in &mut self.faces {
                f.element.selected = selected && f.material != 0;
            }
        }
        if types & 1 != 0 {
            for e in &mut self.edges {
                e.element.selected = selected;
            }
        }
    }

    pub fn selection_to_mask(&mut self, mask: u32, mode: Selection) {
        for v in &mut self.vertices {
            v.element.select(mask >> 16, v.element.selected, mode);
        }
        for f in &mut self.faces {
            f.element.select(mask >> 8, f.element.selected, mode);
        }
        for e in &mut self.edges {
            e.element.select(mask, e.element.selected, mode);
        }
    }

    pub fn faces_to_vertices(&mut self) {
        self.all_to_selection(false, 4);
        for face in 0..self.faces.len() {
            if self.faces[face].element.selected {
                let first = self.faces[face].edge as usize;
                let mut half = first;
                loop {
                    let vertex = self.vertex_id(half);
                    self.vertices[vertex].element.selected = true;
                    half = self.next_face_edge(half);
                    if half == first {
                        break;
                    }
                }
            }
        }
    }

    pub fn vertices_to_faces_edges(&mut self) {
        for edge in &mut self.edges {
            edge.element.selected = self.vertices[edge.vertex[0] as usize].element.selected
                && self.vertices[edge.vertex[1] as usize].element.selected;
        }
        for face in 0..self.faces.len() {
            if self.faces[face].material == 0 {
                continue;
            }
            let first = self.faces[face].edge as usize;
            let mut half = first;
            let selected = loop {
                if !self.vertices[self.vertex_id(half)].element.selected {
                    break false;
                }
                half = self.next_face_edge(half);
                if half == first {
                    break true;
                }
            };
            self.faces[face].element.selected = selected;
        }
    }

    pub fn select_cube(
        &mut self,
        center: [f32; 3],
        half_size: [f32; 3],
        mask: u32,
        mode: Selection,
    ) {
        for v in &mut self.vertices {
            v.element.selected =
                (0..3).all(|i| (v.attributes[0][i] - center[i]).abs() <= half_size[i]);
        }
        self.vertices_to_faces_edges();
        self.selection_to_mask(mask, mode);
    }

    /// Attribute indices use the packed default layout (UV0 is index 4).
    pub fn transform(&mut self, matrix: &Matrix, source: usize, destination: usize, add: bool) {
        for vertex in &mut self.vertices {
            if !vertex.element.selected {
                continue;
            }
            let v = vertex.attributes[source];
            let w = if source == 3 { 1. } else { v[3] };
            let transformed = std::array::from_fn::<_, 3, _>(|i| {
                v[0] * matrix[0][i] + v[1] * matrix[1][i] + v[2] * matrix[2][i] + w * matrix[3][i]
            });
            for (out, value) in vertex.attributes[destination][..3]
                .iter_mut()
                .zip(transformed)
            {
                if add {
                    *out += value;
                } else {
                    *out = value;
                }
            }
            if !add {
                vertex.attributes[destination][3] = v[3];
            }
        }
    }

    pub fn crease(&mut self, selection_type: i32, mask: u32, mode: Selection, what: i32) {
        assert_eq!(what & 1, 0);
        for half in 0..self.edges.len() * 2 {
            let selected = match selection_type {
                0 => {
                    self.faces[self.face_id(half)].element.selected
                        && !self.faces[self.face_id(half ^ 1)].element.selected
                }
                1 => self.edges[half >> 1].element.selected,
                _ => self.faces[self.face_id(half)].element.selected,
            };
            if !selected {
                continue;
            }
            if self.edges[half >> 1].crease == 0 {
                for direction in 0..2 {
                    let mut edge = half ^ direction;
                    let original = self.vertex_id(edge);
                    let first = self.vertices[original].first;
                    let mut got = 0;
                    let mut cursor = edge;
                    loop {
                        got |= self.edges[cursor >> 1].crease;
                        cursor = self.next_vertex_edge(cursor);
                        if cursor == edge {
                            break;
                        }
                    }
                    if got != 0 {
                        let copy = self.add_copied_vertex(original);
                        self.vertices[copy].element.select(mask >> 16, true, mode);
                        self.vertices[copy].first = first;
                        self.vertices[original].next = copy as i32;
                        self.vertices[copy].attributes = self.vertices[original].attributes;
                        loop {
                            assert_eq!(self.vertex_id(edge), original);
                            self.edges[edge >> 1].vertex[edge & 1] = copy as i32;
                            edge = self.next_vertex_edge(edge);
                            if self.edges[edge >> 1].crease != 0 {
                                break;
                            }
                        }
                    }
                }
            }
            self.edges[half >> 1].crease |= what;
        }
    }

    pub fn uncrease(&mut self, use_edges: bool, what: i32) {
        assert_eq!(what & 1, 0);
        for edge in 0..self.edges.len() {
            let selected = if use_edges {
                self.edges[edge].element.selected
            } else {
                self.is_border_edge(edge * 2, 0)
            };
            if selected {
                self.edges[edge].crease &= !what;
            }
        }
    }

    pub fn cube(tessellation: [usize; 3], flags: u32, srt: [f32; 9]) -> Self {
        assert!(tessellation.iter().all(|&n| n > 0));
        let mut mesh = Self::ring(
            4,
            0,
            std::f32::consts::SQRT_2 / 2.,
            std::f32::consts::TAU / 8.,
            0,
        );
        mesh.faces[0].element.selected = true;
        mesh.extrude(0x10000, Selection::Add, 0);
        let mut mat = identity();
        mat[3][2] = 1.;
        mesh.mask_to_selection(0x10000);
        mesh.transform(&mat, 0, 0, false);
        mat[3] = [0.5, 0.5, 0., 1.];
        mesh.all_to_selection(true, 7);
        mesh.transform(&mat, 0, 0, false);
        mesh.transform(&identity(), 0, 4, false);
        let scale = if flags & 8 != 0 {
            [srt[0], srt[1], srt[2]]
        } else {
            [1.; 3]
        };
        for (axis, &steps) in tessellation.iter().enumerate() {
            let mut size = [1025.; 3];
            size[axis] = 0.1;
            mesh.select_cube([1.; 3], size, 0, Selection::Add);
            mat = identity();
            mat[3][axis] = 1.;
            for _ in 1..steps {
                // The original second argument is a selection mode, not a mask.
                // 0x100 matches none of the switch cases and leaves masks intact.
                mesh.extrude(0x10000, Selection::Unchanged, 0);
                mesh.faces_to_vertices();
                mesh.transform(&mat, 0, 0, false);
                mesh.transform(&mat, 4, 4, false);
            }
        }
        mat = identity();
        for axis in 0..3 {
            mat[axis][axis] = 1. / tessellation[axis] as f32;
        }
        mesh.all_to_selection(true, 7);
        mesh.transform(&mat, 4, 4, false);
        mat[3] = [-0.5, -0.5, -0.5, 1.];
        mesh.transform(&mat, 0, 0, false);
        for side in 0..6 {
            let sign = if side & 1 != 0 { 1. } else { -1. };
            let mut center = [0.; 3];
            center[side / 2] = sign * 0.5;
            let mut size = [1025.; 3];
            size[side / 2] = 0.001;
            mesh.select_cube(center, size, 0, Selection::Add);
            mesh.crease(
                0,
                0,
                Selection::Add,
                if flags & 1 != 0 { 0x1fe } else { 32 },
            );
            mesh.faces_to_vertices();
            mesh.selection_to_mask(0x10100 << side, Selection::Set);
            mat = [[0.; 4]; 4];
            match side / 2 {
                0 => {
                    mat[1][1] = -scale[1];
                    mat[2][0] = sign * scale[2];
                }
                1 => {
                    mat[0][0] = scale[0];
                    mat[2][1] = -sign * scale[2];
                }
                _ => {
                    mat[0][0] = -sign * scale[0];
                    mat[1][1] = -scale[1];
                }
            }
            mat[3][0] = 0.5 * (mat[0][0].abs() + mat[2][0].abs());
            mat[3][1] = 0.5 * (mat[1][1].abs() + mat[2][1].abs());
            mesh.transform(&mat, 0, 4, false);
        }
        if flags & 2 != 0 {
            mat = identity();
            mat[3][0] = scale[2];
            mesh.mask_to_selection(0x100000);
            mesh.transform(&mat, 4, 4, false);
            mat[3][0] += scale[0];
            mesh.mask_to_selection(0x020000);
            mesh.transform(&mat, 4, 4, false);
            mat[3][0] += scale[2];
            mesh.mask_to_selection(0x200000);
            mesh.transform(&mat, 4, 4, false);
        }
        if flags & 0x30 != 0 {
            let mode = (((flags & 0x30) >> 4) - 1) | ((flags & 0xc0) >> 4);
            let mut vertices = [0; 8];
            for (i, index) in vertices.iter_mut().enumerate() {
                *index = mesh.add_vertex();
                mesh.vertices[*index].attributes[0] = [
                    0.5 - ((i & 1) as f32),
                    0.5 - (((i >> 1) & 1) as f32),
                    0.5 - (((i >> 2) & 1) as f32),
                    1.,
                ];
            }
            mesh.collision.push(Collision {
                vertices,
                mode: mode as i32,
                code: 0,
                condition: 0,
                value: 0,
                output: 0,
                events: [None; 2],
            });
        }
        mesh.all_to_selection(true, 7);
        if flags & 4 != 0 {
            mat = identity();
            mat[3][1] = 0.5;
            mesh.transform(&mat, 0, 0, false);
        }
        mesh.transform(&srt_matrix(&srt), 0, 0, false);
        mesh.parts.push(0);
        mesh
    }
    /// Split one face between the origins of two half-edges, preserving source IDs.
    pub fn split_face(&mut self, a: usize, b: usize, mask: u32, mode: Selection) {
        assert_ne!(a, b);
        let original = self.face_id(a);
        assert_eq!(original, self.face_id(b));
        let previous_a = self.previous_face_edge(a);
        let previous_b = self.previous_face_edge(b);
        let index = self.edges.len();
        let new_face = self.faces.len();
        let mut edge = self.edges[a >> 1].clone();
        edge.crease = 0;
        edge.element.select(mask, true, mode);
        edge.vertex = [self.vertex_id(b) as i32, self.vertex_id(a) as i32];
        edge.face = [original as i32; 2];
        edge.next = [a as i32, b as i32];
        edge.previous = [previous_b as i32, previous_a as i32];
        edge.temp = [-1; 2];
        self.edges.push(edge);
        self.edges[a >> 1].previous[a & 1] = (2 * index) as i32;
        self.edges[previous_a >> 1].next[previous_a & 1] = (2 * index + 1) as i32;
        self.edges[b >> 1].previous[b & 1] = (2 * index + 1) as i32;
        self.edges[previous_b >> 1].next[previous_b & 1] = (2 * index) as i32;
        let mut face = self.faces[original].clone();
        face.element.select(mask >> 8, true, mode);
        face.edge = b as i32;
        self.faces.push(face);
        self.faces[original].edge = a as i32;
        let mut half = b;
        loop {
            self.edges[half >> 1].face[half & 1] = new_face as i32;
            half = self.next_face_edge(half);
            if half == b {
                break;
            }
        }
    }

    /// Rebuild the shared-position vertex cycle after inserting a bridge.
    pub fn fix_vertex_cycle(&mut self, half: usize) {
        let start = self.vertices[self.vertex_id(half)].reindex as usize;
        let mut vertex = start;
        self.vertices[vertex].first = start as i32;
        self.vertices[vertex].next = start as i32;
        let mut edge = half;
        loop {
            self.edges[edge >> 1].vertex[edge & 1] = vertex as i32;
            edge = self.next_vertex_edge(edge);
            let next = self.vertices[self.vertex_id(edge)].reindex as usize;
            if self.edges[edge >> 1].crease != 0 {
                if next != vertex && next != start {
                    self.vertices[vertex].next = next as i32;
                    self.vertices[next].first = start as i32;
                    self.vertices[next].next = start as i32;
                }
                vertex = next;
            }
            if edge == half {
                break;
            }
        }
    }

    /// Original SplitBridge, returning the two resulting vertex IDs and optional
    /// source/destination pair for a vertex duplicated at a crease.
    pub fn split_bridge(
        &mut self,
        a: usize,
        d: usize,
        vertex: usize,
        mask: u32,
        mode: Selection,
    ) -> (usize, usize, Option<(usize, usize)>) {
        self.vertices[vertex].element.select(mask >> 16, true, mode);
        self.vertices[vertex].first = vertex as i32;
        self.vertices[vertex].next = vertex as i32;
        let index = self.edges.len();
        let a = a ^ 1;
        let mut previous = self.previous_face_edge(d);
        let mut next = self.next_face_edge(a);
        let other = self.vertex_id(next);
        assert_eq!(
            self.vertices[other].first,
            self.vertices[self.vertex_id(d)].first
        );
        let mut crease = 0;
        let mut last_crease = None;
        if next == d {
            previous = index * 2 + 1;
            next = index * 2;
        } else {
            let mut i = next;
            while i != d {
                if self.edges[i >> 1].crease != 0 {
                    crease |= self.edges[i >> 1].crease;
                    last_crease = Some(i);
                }
                i = self.previous_vertex_edge(i);
            }
        }
        let mut edge = self.edges[a >> 1].clone();
        edge.crease = crease;
        edge.element.select(mask, true, mode);
        edge.vertex = [vertex as i32, other as i32];
        edge.face = [self.face_id(d) as i32, self.face_id(a) as i32];
        edge.next = [d as i32, next as i32];
        edge.previous = [previous as i32, a as i32];
        edge.temp = [-1; 2];
        self.edges.push(edge);
        self.edges[d >> 1].previous[d & 1] = (index * 2) as i32;
        self.edges[previous >> 1].next[previous & 1] = (index * 2) as i32;
        self.edges[next >> 1].previous[next & 1] = (index * 2 + 1) as i32;
        self.edges[a >> 1].next[a & 1] = (index * 2 + 1) as i32;
        let duplicated = last_crease.map(|half| {
            let source = self.vertex_id(self.previous_vertex_edge(half) ^ 1);
            let copy = self.add_copied_vertex(source);
            self.vertices[copy].element.select(mask >> 16, true, mode);
            self.edges[half >> 1].vertex[half & 1] = copy as i32;
            (source, copy)
        });
        self.fix_vertex_cycle(index * 2);
        self.fix_vertex_cycle(index * 2 + 1);
        (self.vertex_id(index * 2), self.vertex_id(next), duplicated)
    }

    pub fn is_border_edge(&self, half: usize, mode: i32) -> bool {
        let selected = self.faces[self.face_id(half)].element.selected;
        let mut opposite = self.faces[self.face_id(half ^ 1)].element.selected;
        if self.edges[half >> 1].crease & 6 != 0 || mode != 0 {
            opposite = false;
        }
        selected && !opposite
    }

    pub fn extrude(&mut self, mask: u32, selection: Selection, mode: i32) {
        let count = self.edges.len() * 2;
        for half in 0..count {
            self.edges[half >> 1].temp[half & 1] = -1;
            if self.is_border_edge(half, mode) {
                self.edges[half >> 1].temp[half & 1] = 0;
                let mut next = self.next_vertex_edge(half);
                while !self.is_border_edge(next ^ 1, mode) {
                    next = self.next_vertex_edge(next);
                }
                let copy = self.add_copied_vertex(self.vertex_id(half));
                let (_, _, duplicate) = self.split_bridge(next, half, copy, mask, selection);
                self.vertices[copy].attributes = self.vertices[self.vertex_id(half)].attributes;
                if let Some((source, destination)) = duplicate {
                    self.vertices[destination].attributes = self.vertices[source].attributes;
                }
            }
        }
        for half in 0..count {
            if self.edges[half >> 1].temp[half & 1] == 0 {
                self.split_face(
                    self.next_face_edge(self.next_face_edge(half)),
                    self.previous_face_edge(half),
                    0,
                    Selection::Add,
                );
                let face = self.faces.last_mut().unwrap();
                face.element.mask = 0;
                face.element.selected = false;
                face.element.select(mask >> 8, true, selection);
            }
        }
    }

    pub fn vertex_id(&self, half: usize) -> usize {
        self.edges[half >> 1].vertex[half & 1] as usize
    }
    pub fn face_id(&self, half: usize) -> usize {
        self.edges[half >> 1].face[half & 1] as usize
    }
    pub fn next_face_edge(&self, half: usize) -> usize {
        self.edges[half >> 1].next[half & 1] as usize
    }
    pub fn previous_face_edge(&self, half: usize) -> usize {
        self.edges[half >> 1].previous[half & 1] as usize
    }
    pub fn next_vertex_edge(&self, half: usize) -> usize {
        (self.edges[half >> 1].previous[half & 1] ^ 1) as usize
    }
    pub fn previous_vertex_edge(&self, half: usize) -> usize {
        self.edges[half >> 1].next[!half & 1] as usize
    }
    pub fn add_vertex(&mut self) -> usize {
        let index = self.vertices.len();
        self.vertices.push(Vertex::new(index as i32));
        index
    }
    /// Original AddCopiedVert copies topology/weights, but not vertex attributes.
    pub fn add_copied_vertex(&mut self, source: usize) -> usize {
        let index = self.vertices.len();
        let mut vertex = self.vertices[source].clone();
        vertex.reindex = index as i32;
        vertex.attributes = [[0.; 4]; 5];
        self.vertices.push(vertex);
        index
    }
    pub fn all_to_mask(&mut self, mask: u32, mode: Selection) {
        for v in &mut self.vertices {
            v.element.select(mask >> 16, true, mode);
        }
        for f in &mut self.faces {
            if f.material != 0 {
                f.element.select(mask >> 8, true, mode);
            }
        }
        for e in &mut self.edges {
            e.element.select(mask, true, mode);
        }
    }
    pub fn mask_to_selection(&mut self, mask: u32) {
        for v in &mut self.vertices {
            if mask & 0xff0000 != 0 {
                v.element.selected = v.element.mask as u32 & (mask >> 16) != 0;
            }
        }
        for f in &mut self.faces {
            if mask & 0xff00 != 0 {
                f.element.selected = f.element.mask as u32 & (mask >> 8) != 0;
            }
        }
        for e in &mut self.edges {
            if mask & 0xff != 0 {
                e.element.selected = e.element.mask as u32 & mask != 0;
            }
        }
    }
    /// Two-sided ring used by the original cube and cylinder generators.
    pub fn ring(segments: usize, mask: u32, radius: f32, phase: f32, arc: usize) -> Self {
        assert!(segments >= 2 && arc < segments);
        let count = if arc > 0 {
            segments - arc + 2
        } else {
            segments
        };
        let mut mesh = Self {
            faces: vec![
                Face {
                    edge: 0,
                    ..Face::default()
                },
                Face {
                    edge: 1,
                    ..Face::default()
                },
            ],
            ..Self::default()
        };
        for i in 0..count {
            mesh.add_vertex();
            let position = &mut mesh.vertices[i].attributes[0];
            if i != count - 1 || arc == 0 {
                let angle = phase + i as f32 * std::f32::consts::TAU / segments as f32;
                position[0] = angle.sin() * radius;
                position[1] = -angle.cos() * radius;
            }
            position[3] = 1.;
            mesh.vertices[i].attributes[4][0] = i as f32 / count as f32;
            mesh.vertices[i].attributes[4][3] = 1.;
            let next = ((i + 1) % count) as i32;
            let previous = ((i + count - 1) % count) as i32;
            mesh.edges.push(Edge {
                next: [2 * next, 2 * previous + 1],
                previous: [2 * previous, 2 * next + 1],
                face: [0, 1],
                vertex: [i as i32, next],
                ..Edge::default()
            });
        }
        mesh.all_to_mask(mask, Selection::Add);
        mesh
    }
}
