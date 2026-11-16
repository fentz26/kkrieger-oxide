//! CPU half of original EngMesh::FromGenMesh and PrepareJob.
use crate::{material::Program, mesh::Mesh};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SilhouetteEdge {
    pub vertices: [u16; 2],
    pub faces: [i32; 2],
}
#[derive(Clone, Debug)]
pub struct Job {
    pub material: usize,
    pub program: Program,
    pub bounds: [[f32; 3]; 2],
    pub vertices: Vec<usize>,
    pub indices: Vec<usize>,
    pub planes: Vec<[f32; 4]>,
    pub face_map: Vec<usize>,
    pub edges: Vec<SilhouetteEdge>,
    /// Source leaves the final part entry unwritten; it has no readers.
    pub face_part_end: Vec<Option<usize>>,
    pub edge_part_end: Vec<Option<usize>>,
}
#[derive(Clone, Debug)]
pub struct PreparedMesh {
    pub materials: Vec<crate::mesh::Material>,
    pub vertices: Vec<Vertex>,
    pub part_bounds: Vec<[f32; 6]>,
    pub jobs: Vec<Job>,
    pub pass_jobs: Vec<Vec<usize>>,
}

fn ring(mesh: &Mesh, face: usize) -> Vec<usize> {
    let first = mesh.faces[face].edge as usize;
    let mut edge = first;
    let mut out = Vec::new();
    loop {
        out.push(edge);
        edge = mesh.next_face_edge(edge);
        if edge == first {
            return out;
        }
    }
}
fn bounds(mesh: &Mesh, faces: impl Iterator<Item = usize>) -> [[f32; 3]; 2] {
    let mut result = [[1e20f32; 3], [-1e20f32; 3]];
    for face in faces {
        for edge in ring(mesh, face) {
            let p = mesh.vertices[mesh.vertex_id(edge)].attributes[0];
            for (axis, &v) in p[..3].iter().enumerate() {
                result[0][axis] = result[0][axis].min(v);
                result[1][axis] = result[1][axis].max(v);
            }
        }
    }
    result
}
fn plane(mesh: &Mesh, face: usize) -> [f32; 4] {
    let mut edge = mesh.faces[face].edge as usize;
    let first = edge;
    let (normal, length) = loop {
        let p = mesh.vertices[mesh.vertex_id(mesh.previous_face_edge(edge))].attributes[0];
        let q = mesh.vertices[mesh.vertex_id(edge)].attributes[0];
        let r = mesh.vertices[mesh.vertex_id(mesh.next_face_edge(edge))].attributes[0];
        let a: [f32; 3] = std::array::from_fn(|i| q[i] - p[i]);
        let b: [f32; 3] = std::array::from_fn(|i| r[i] - p[i]);
        let n = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let length = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
        edge = mesh.next_face_edge(edge);
        // Preserve the source's greater-than condition, including degenerate faces.
        if !(length > 1e-40 && edge != first) {
            break (n, length);
        }
    };
    let n = if length >= 1e-40 {
        let scale = 1. / length.sqrt();
        normal.map(|v| v * scale)
    } else {
        [1., 0., 0.]
    };
    let p = mesh.vertices[mesh.vertex_id(mesh.faces[face].edge as usize)].attributes[0];
    [n[0], n[1], n[2], -(n[0] * p[0] + n[1] * p[1] + n[2] * p[2])]
}

impl PreparedMesh {
    /// Programs are supplied per material slot in original pass order. Resolve
    /// MaterialSource::Default and Operator before calling; slot zero is unused.
    pub fn from_mesh(mesh: &mut Mesh, programs: &[Vec<Program>]) -> Result<Self, &'static str> {
        if programs.len() != mesh.materials.len() {
            return Err("material slot count mismatch");
        }
        if !mesh.got_normals {
            mesh.calculate_normals(false, 7);
            mesh.got_normals = true;
        }
        let vertices = mesh
            .vertices
            .iter()
            .map(|v| {
                let a = &v.attributes;
                let c = a[3].map(|v| (v * 255.).floor().clamp(0., 255.) as u32);
                Vertex {
                    position: a[0][..3].try_into().unwrap(),
                    normal: a[1][..3].try_into().unwrap(),
                    tangent: a[2][..3].try_into().unwrap(),
                    color: c[0] << 16 | c[1] << 8 | c[2] | c[3] << 24,
                    uv: [a[4][0], a[4][1]],
                }
            })
            .collect();
        let part_bounds = mesh
            .parts
            .iter()
            .enumerate()
            .map(|(i, &start)| {
                let end = mesh.parts.get(i + 1).copied().unwrap_or(mesh.faces.len());
                let bb = bounds(mesh, start..end);
                std::array::from_fn(|i| {
                    if i % 2 == 0 {
                        (bb[1][i / 2] + bb[0][i / 2]) * 0.5
                    } else {
                        (bb[1][i / 2] - bb[0][i / 2]) * 0.5
                    }
                })
            })
            .collect();
        let mut prepared = Self {
            materials: mesh.materials.clone(),
            vertices,
            part_bounds,
            jobs: Vec::new(),
            pass_jobs: vec![Vec::new(); programs.len()],
        };
        for (material, passes) in programs.iter().enumerate().skip(1) {
            let mut reused = Vec::new();
            for &program in passes {
                let id = if let Some(&(_, id)) = reused.iter().find(|&&(p, _)| p == program) {
                    id
                } else {
                    let id = prepared.jobs.len();
                    prepared.jobs.push(Job::prepare(mesh, material, program));
                    reused.push((program, id));
                    id
                };
                prepared.pass_jobs[material].push(id);
            }
        }
        Ok(prepared)
    }
}
impl Job {
    pub fn prepare(mesh: &mut Mesh, material: usize, program: Program) -> Self {
        let bounds = bounds(
            mesh,
            (0..mesh.faces.len())
                .filter(|&i| mesh.faces[i].material as usize == material && material != 0),
        );
        let mut job = Self {
            material,
            program,
            bounds,
            vertices: Vec::new(),
            indices: Vec::new(),
            planes: Vec::new(),
            face_map: Vec::new(),
            edges: Vec::new(),
            face_part_end: Vec::new(),
            edge_part_end: Vec::new(),
        };
        if program == Program::Sprites {
            mesh.all_to_selection(true, 2);
            mesh.faces_to_vertices();
            job.vertices = (0..mesh.vertices.len())
                .filter(|&i| {
                    mesh.vertices[i].first == i as i32 && mesh.vertices[i].element.selected
                })
                .collect();
            return job;
        }
        if program == Program::ThickLines {
            for i in 0..mesh.edges.len() {
                let selected = mesh.faces[mesh.face_id(i * 2)].material as usize == material
                    || mesh.faces[mesh.face_id(i * 2 + 1)].material as usize == material;
                mesh.edges[i].element.selected = selected;
                if selected {
                    job.vertices
                        .extend(mesh.edges[i].vertex.map(|i| i as usize));
                }
            }
            return job;
        }
        let shadow = program == Program::Shadow;
        mesh.all_to_selection(false, 7);
        let mut face_count = 0;
        for face in &mut mesh.faces {
            face.temp = face_count;
            if face.material as usize == material && (!shadow || face.element.used) {
                face.element.selected = true;
                face_count += 1;
            }
        }
        let mut edge_count = 0;
        for i in 0..mesh.edges.len() {
            if mesh.faces[mesh.face_id(i * 2)].element.selected
                || mesh.faces[mesh.face_id(i * 2 + 1)].element.selected
            {
                mesh.edges[i].temp[0] = edge_count;
                edge_count += 1;
                for half in [i * 2, i * 2 + 1] {
                    let v = mesh.vertex_id(half);
                    mesh.vertices[v].element.selected = true;
                }
            }
        }
        for (i, v) in mesh.vertices.iter_mut().enumerate() {
            v.temp = job.vertices.len() as i32;
            if v.element.selected {
                job.vertices.push(i);
            }
        }
        if shadow {
            job.planes.push([0.; 4]);
            for i in 0..mesh.faces.len() {
                if mesh.faces[i].element.selected {
                    job.planes.push(plane(mesh, i));
                }
            }
        }
        job.face_part_end = vec![None; mesh.parts.len()];
        job.edge_part_end = vec![None; mesh.parts.len()];
        let mut current_part = 0;
        let mut face_count = 0;
        for i in 0..mesh.faces.len() {
            while current_part + 1 < mesh.parts.len() && i >= mesh.parts[current_part + 1] {
                job.face_part_end[current_part] = Some(face_count);
                job.edge_part_end[current_part] = Some(job.edges.len());
                current_part += 1;
            }
            if !mesh.faces[i].element.selected {
                continue;
            }
            let edges = ring(mesh, i);
            let verts: Vec<_> = edges
                .iter()
                .map(|&e| mesh.vertices[mesh.vertex_id(e)].temp as usize)
                .collect();
            for n in 2..verts.len() {
                job.indices.extend(
                    [verts[0], verts[n - 1], verts[n]].map(|v| v * if shadow { 2 } else { 1 }),
                );
                if shadow {
                    job.face_map.push(face_count + 1);
                }
            }
            if shadow {
                for e in edges {
                    if !mesh.edges[e / 2].element.selected {
                        mesh.edges[e / 2].element.selected = true;
                        job.edges.push(SilhouetteEdge {
                            vertices: [
                                (mesh.vertices[mesh.vertex_id(e)].temp * 2) as u16,
                                (mesh.vertices[mesh.vertex_id(mesh.next_face_edge(e))].temp * 2)
                                    as u16,
                            ],
                            faces: [
                                mesh.faces[mesh.face_id(e)].temp + 1,
                                mesh.faces[mesh.face_id(e ^ 1)].temp + 1,
                            ],
                        });
                    }
                }
            }
            face_count += 1;
        }
        job
    }
}

pub struct Assets {
    pub meshes: Vec<std::rc::Rc<PreparedMesh>>,
    pub materials: std::collections::BTreeMap<usize, std::rc::Rc<crate::material::Material>>,
}

/// Resolve original material programs and prepare geometry as a shared batch.
/// This is CPU preparation, not GPU shader compilation or texture upload.
pub fn generate_many(
    doc: &crate::kx::Document<'_>,
    roots: &[usize],
) -> Result<Assets, Box<dyn std::error::Error>> {
    generate_many_with_materials(doc, roots, &[])
}

/// Prepare geometry and extra effect-material roots through one shared material
/// batch, preserving cache identity when meshes and effects use the same pass.
pub fn generate_many_with_materials(
    doc: &crate::kx::Document<'_>,
    roots: &[usize],
    extra_material_roots: &[usize],
) -> Result<Assets, Box<dyn std::error::Error>> {
    use crate::{material, mesh::MaterialSource, mesh_graph};
    use std::{
        collections::{BTreeMap, BTreeSet},
        rc::Rc,
    };
    let meshes = mesh_graph::generate_many(doc, roots)?;
    let material_roots: Vec<_> = meshes
        .iter()
        .flat_map(|m| m.materials.iter())
        .filter_map(|m| match m.source {
            MaterialSource::Operator(i) => Some(i),
            _ => None,
        })
        .chain(extra_material_roots.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let materials: BTreeMap<_, _> = material_roots
        .iter()
        .copied()
        .zip(material::generate_many(doc, &material_roots)?)
        .collect();
    let mut cache = BTreeMap::new();
    let mut output = Vec::with_capacity(meshes.len());
    for (&root, mesh) in roots.iter().zip(meshes) {
        if let Some(prepared) = cache.get(&root) {
            output.push(Rc::clone(prepared));
            continue;
        }
        let programs: Vec<_> = mesh
            .materials
            .iter()
            .map(|m| match m.source {
                MaterialSource::Null => Vec::new(),
                // genoverlay.cpp: DefaultMat has one ENGU_LIGHT / MPP_STATIC pass.
                MaterialSource::Default => vec![Program::Static],
                MaterialSource::Operator(i) => {
                    materials[&i].passes.iter().map(|p| p.program).collect()
                }
            })
            .collect();
        let mut mesh = Rc::try_unwrap(mesh).unwrap_or_else(|m| (*m).clone());
        let prepared = Rc::new(PreparedMesh::from_mesh(&mut mesh, &programs)?);
        cache.insert(root, Rc::clone(&prepared));
        output.push(prepared);
    }
    Ok(Assets {
        meshes: output,
        materials,
    })
}
