//! Original geometry DAG evaluation. Materials and events remain graph handles
//! for their own runtime subsystems; unsupported geometry is always an error.
use crate::{
    kx::Document,
    mesh::{Material, MaterialSource, Mesh, Selection, srt_matrix},
};
use std::{fmt, rc::Rc};

#[derive(Debug)]
pub struct Error {
    pub operator: usize,
    pub class: u16,
    pub reason: &'static str,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mesh op {} (0x{:02x}): {}",
            self.operator, self.class, self.reason
        )
    }
}
impl std::error::Error for Error {}

pub fn implemented(id: u16) -> bool {
    matches!(id,0x81|0x82|0x85..=0x89|0x8c|0x8f..=0x93|0x95|0x96|0x9a|0x9c|0x9d|0xa0|0xa5|0xa6|0xa7|0xab|0xad|0xaf|0xb0|0xb3)
}

fn mode(value: u32) -> Selection {
    match value {
        0 => Selection::Add,
        1 => Selection::Subtract,
        2 => Selection::Set,
        3 => Selection::SetNot,
        _ => Selection::Unchanged,
    }
}
fn select(mesh: &mut Mesh, mask: u32) {
    if mask & 0x80000000 == 0 {
        if mask == 0 {
            mesh.all_to_selection(true, 7);
        } else {
            mesh.mask_to_selection(mask);
        }
    }
}
fn attribute(index: u32) -> Option<usize> {
    match index {
        0..=3 => Some(index as usize),
        5 => Some(4),
        _ => None,
    }
}

fn geometry_inputs(inputs: &[usize], id: u16) -> &[usize] {
    if id == 0x8f {
        &inputs[..inputs.len().min(1)]
    } else {
        inputs
    }
}

pub fn generate_many(doc: &Document<'_>, roots: &[usize]) -> Result<Vec<Rc<Mesh>>, Error> {
    let fail = |i, reason| Error {
        operator: i,
        class: doc
            .operators
            .get(i)
            .map(|o| doc.classes[o.class].id)
            .unwrap_or(0),
        reason,
    };
    let mut needed = vec![false; doc.operators.len()];
    let mut pending = roots.to_vec();
    while let Some(i) = pending.pop() {
        let op = doc
            .operators
            .get(i)
            .ok_or_else(|| fail(i, "root out of range"))?;
        if needed[i] {
            continue;
        }
        if !implemented(doc.classes[op.class].id) {
            return Err(fail(i, "operator has not been ported"));
        }
        if op.animation != [1] {
            return Err(fail(i, "animated mesh requires an evaluation environment"));
        }
        needed[i] = true;
        pending.extend(
            geometry_inputs(&op.inputs, doc.classes[op.class].id)
                .iter()
                .copied(),
        );
    }
    let mut users = vec![0usize; doc.operators.len()];
    for &root in roots {
        users[root] += 1;
    }
    for (i, op) in doc.operators.iter().enumerate().filter(|(i, _)| needed[*i]) {
        for &input in geometry_inputs(&op.inputs, doc.classes[op.class].id) {
            if input >= i {
                return Err(fail(i, "input is not topologically ordered"));
            }
            users[input] += 1;
        }
    }
    let mut texture_users = vec![0usize; doc.operators.len()];
    for (i, op) in doc.operators.iter().enumerate().filter(|(i, _)| needed[*i]) {
        if doc.classes[op.class].id == 0x8f {
            if op.inputs.len() != 2 {
                return Err(fail(i, "displacement needs mesh and bitmap inputs"));
            }
            texture_users[op.inputs[1]] += 1;
        }
    }
    let texture_roots: Vec<_> = texture_users
        .iter()
        .enumerate()
        .filter_map(|(i, &users)| (users != 0).then_some(i))
        .collect();
    let textures = crate::texture::generate_many(doc, &texture_roots, 0).map_err(|e| Error {
        operator: e.operator,
        class: e.class,
        reason: e.reason,
    })?;
    let mut texture_cache = vec![None; doc.operators.len()];
    for (id, texture) in texture_roots.into_iter().zip(textures) {
        texture_cache[id] = Some(texture);
    }
    let mut cache: Vec<Option<Rc<Mesh>>> = vec![None; doc.operators.len()];
    let mut noise = None;
    for (i, op) in doc.operators.iter().enumerate().filter(|(i, _)| needed[*i]) {
        let id = doc.classes[op.class].id;
        if !op.links.is_empty() && id != 0x96 && id != 0x9c {
            return Err(fail(i, "unexpected linked geometry input"));
        }
        let integer = |n| {
            op.parameters
                .get(n)
                .copied()
                .ok_or_else(|| fail(i, "missing parameter"))
        };
        let float = |n| integer(n).map(f32::from_bits);
        let floats = |start: usize, count: usize| -> Result<Vec<f32>, Error> {
            (start..start + count).map(float).collect()
        };
        let size = |n| -> Result<usize, Error> {
            let value = integer(n)? as i32;
            if value <= 0 {
                Err(fail(i, "invalid tessellation"))
            } else {
                Ok(value as usize)
            }
        };
        let mut inputs = Vec::with_capacity(op.inputs.len());
        for &input in geometry_inputs(&op.inputs, doc.classes[op.class].id) {
            inputs.push(
                cache[input]
                    .as_ref()
                    .ok_or_else(|| fail(i, "missing input mesh"))?
                    .clone(),
            );
            users[input] -= 1;
            if users[input] == 0 {
                cache[input] = None;
            }
        }
        let mut mesh = if inputs.is_empty() {
            Rc::new(Mesh::default())
        } else {
            inputs.remove(0)
        };
        if op.inputs.is_empty() && !matches!(id, 0x81 | 0x82 | 0x9c | 0x9d | 0xaf | 0xb3) {
            return Err(fail(i, "missing input mesh"));
        }
        if !inputs.is_empty() && id != 0x92 {
            return Err(fail(i, "unexpected extra mesh input"));
        }
        let mesh_mut = Rc::make_mut(&mut mesh);
        match id {
            0x9d => {
                *mesh_mut = Mesh::grid(integer(0)?, size(1)?, size(2)?);
            }

            0x9a => {
                select(mesh_mut, integer(0)?.wrapping_shl(8));
                let count = integer(3)? as i32;
                if count < 0 {
                    return Err(fail(i, "negative extrusion count"));
                }
                mesh_mut.extrude_faces(
                    integer(1)?,
                    integer(2)?,
                    count as usize,
                    floats(4, 3)?.try_into().unwrap(),
                    floats(7, 3)?.try_into().unwrap(),
                    floats(10, 3)?.try_into().unwrap(),
                );
            }

            0xa5 => {
                mesh_mut.uv_projection(
                    integer(0)?,
                    floats(1, 9)?.try_into().unwrap(),
                    integer(10)? as i32,
                );
            }

            0xad => {
                mesh_mut.light_vertices(
                    floats(0, 3)?.try_into().unwrap(),
                    floats(3, 2)?.try_into().unwrap(),
                    integer(5)?,
                    float(6)?,
                    float(7)?,
                    integer(8)? as i32,
                );
            }
            0xaf => {
                *mesh_mut = Mesh::light_slot(
                    floats(0, 3)?.try_into().unwrap(),
                    integer(3)?,
                    float(4)?,
                    float(5)?,
                );
            }

            0xa6 => {
                select(mesh_mut, integer(0)?.wrapping_shl(16));
                mesh_mut.center(integer(1)?);
            }
            0xa7 => {
                mesh_mut.auto_collision(
                    float(0)?,
                    integer(1)? as i32,
                    [size(2)?, size(3)?, size(4)?],
                    integer(5)?,
                );
            }

            0x95 => {
                *mesh_mut = mesh_mut.multiplied(
                    floats(0, 9)?.try_into().unwrap(),
                    integer(9)? as i32,
                    integer(10)?,
                    floats(11, 2)?.try_into().unwrap(),
                    floats(13, 3)?.try_into().unwrap(),
                    float(16)?,
                );
            }

            0x91 => {
                select(mesh_mut, integer(0)?.wrapping_shl(16));
                mesh_mut.perlin(
                    &srt_matrix(&floats(1, 9)?.try_into().unwrap()),
                    floats(10, 3)?.try_into().unwrap(),
                    noise.get_or_insert_with(crate::random::Noise::new),
                );
            }
            0x8f => {
                select(mesh_mut, integer(0)?.wrapping_shl(16));
                mesh_mut.need_all_normals();
                let texture = op.inputs[1];
                mesh_mut.displace(
                    texture_cache[texture]
                        .as_ref()
                        .ok_or_else(|| fail(i, "missing displacement bitmap"))?,
                    floats(1, 3)?.try_into().unwrap(),
                );
                mesh_mut.got_normals = false;
                texture_users[texture] -= 1;
                if texture_users[texture] == 0 {
                    texture_cache[texture] = None;
                }
            }

            0x81 => {
                *mesh_mut = Mesh::cube(
                    [size(0)?, size(1)?, size(2)?],
                    integer(3)?,
                    floats(4, 9)?.try_into().unwrap(),
                )
            }
            0x82 => {
                let segments = size(0)?;
                if segments < 3 {
                    return Err(fail(i, "cylinder needs at least three segments"));
                }
                *mesh_mut = Mesh::cylinder(
                    segments,
                    size(1)?,
                    integer(2)?,
                    size(3)?,
                    op.integer(4).unwrap_or(0).max(0) as usize,
                );
            }
            0x85 => {
                select(mesh_mut, 0);
                mesh_mut.all_to_mask(integer(0)?, mode(integer(1)?));
            }
            0x86 => {
                select(mesh_mut, 0);
                let center = floats(2, 3)?.try_into().unwrap();
                let mut half_size: [f32; 3] = floats(5, 3)?.try_into().unwrap();
                for size in &mut half_size {
                    *size *= 0.5;
                }
                mesh_mut.select_cube(center, half_size, 0, Selection::Add);
                let selection = integer(1)?;
                if selection & 4 != 0 {
                    mesh_mut.faces_to_vertices();
                }
                mesh_mut.selection_to_mask(integer(0)?, mode(selection & 3));
            }
            0x87 => {
                select(mesh_mut, integer(0)?.wrapping_shl(8));
                let count = integer(2)? as i32;
                if count < 0 {
                    return Err(fail(i, "negative subdivision count"));
                }
                for _ in 0..count {
                    mesh_mut.subdivide(float(1)?);
                }
                mesh_mut.got_normals = false;
            }
            0x88 | 0x89 => {
                select(mesh_mut, integer(0)?.wrapping_shl(16));
                let (source, destination) = if id == 0x89 {
                    (integer(10)?, integer(11)?)
                } else {
                    (0, 0)
                };
                if source == 1 || source == 2 {
                    mesh_mut.need_all_normals();
                }
                if let (Some(source), Some(target)) =
                    (attribute(source), attribute(destination & 15))
                {
                    mesh_mut.transform(
                        &srt_matrix(&floats(1, 9)?.try_into().unwrap()),
                        source,
                        target,
                        destination & 16 != 0,
                    );
                }
                if id == 0x88 || destination <= 2 {
                    mesh_mut.got_normals = false;
                }
            }
            0x8c => {
                select(mesh_mut, integer(0)?.wrapping_shl(8));
                mesh_mut.triangulate(size(1)?, 0, Selection::Add, integer(2)? as i32);
                mesh_mut.got_normals = false;
            }
            0x92 => {
                select(mesh_mut, 0);
                for input in inputs {
                    mesh_mut.append(&input);
                }
                mesh_mut.got_normals = false;
            }
            0x90 => {
                let mask = integer(0)?;
                select(mesh_mut, mask.wrapping_shl(8));
                mesh_mut.need_all_normals();
                mesh_mut.bevel(
                    float(1)?,
                    float(2)?,
                    integer(3)? as i32,
                    0,
                    mode(mask.wrapping_shl(8)),
                );
                mesh_mut.got_normals = false;
            }
            0xa0 => {
                select(mesh_mut, integer(0)?.wrapping_shl(16));
                mesh_mut.bend_between(
                    floats(1, 9)?.try_into().unwrap(),
                    floats(10, 9)?.try_into().unwrap(),
                    floats(19, 2)?.try_into().unwrap(),
                    floats(21, 2)?.try_into().unwrap(),
                    integer(23)?,
                );
            }
            0xab => {
                select(mesh_mut, 0);
                mesh_mut.bend_curve(
                    floats(0, 3)?.try_into().unwrap(),
                    floats(3, 3)?.try_into().unwrap(),
                    float(6)?,
                    float(7)?,
                );
            }
            0x93 => {
                select(mesh_mut, integer(0)?.wrapping_shl(8));
                mesh_mut.delete_selected_faces();
            }
            0x96 => {
                select(mesh_mut, integer(0)?.wrapping_shl(8));
                let material = op
                    .links
                    .first()
                    .copied()
                    .flatten()
                    .ok_or_else(|| fail(i, "missing material link"))?;
                let index = mesh_mut.materials.len() as i32;
                mesh_mut.materials.push(Material {
                    source: MaterialSource::Operator(material),
                    pass: integer(1)? as i32,
                });
                for face in &mut mesh_mut.faces {
                    if face.element.selected {
                        face.material = index;
                    }
                }
            }
            0x9c => {
                let cells = Mesh::collision_cube(
                    floats(0, 6)?.try_into().unwrap(),
                    integer(6)? as i32,
                    [size(7)?, size(8)?, size(9)?],
                    [
                        op.integer(10).unwrap_or(0),
                        op.integer(11).unwrap_or(0),
                        op.integer(12).unwrap_or(0),
                        op.integer(13).unwrap_or(0),
                    ],
                    [
                        op.links.first().copied().flatten(),
                        op.links.get(1).copied().flatten(),
                    ],
                );
                if op.inputs.is_empty() {
                    *mesh_mut = cells;
                } else {
                    select(mesh_mut, 0);
                    mesh_mut.append(&cells);
                }
            }
            0xb0 => {
                select(mesh_mut, 0);
                for face in &mut mesh_mut.faces {
                    face.element.used = integer(0)? != 0;
                }
            }
            0xb3 => {
                *mesh_mut = Mesh::ring(3, 0, 0., 0., 0);
                mesh_mut.faces[1].material = 0;
                mesh_mut.vertices[0].next = 1;
                mesh_mut.vertices[1].first = 0;
                mesh_mut.vertices[1].next = 2;
                mesh_mut.vertices[2].first = 0;
                mesh_mut.vertices[2].next = 0;
            }
            _ => unreachable!(),
        }
        cache[i] = Some(mesh);
    }
    roots
        .iter()
        .map(|&root| {
            cache[root]
                .clone()
                .ok_or_else(|| fail(root, "missing output mesh"))
        })
        .collect()
}
