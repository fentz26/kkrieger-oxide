//! Original SetSceneR/AddMesh static-world assembly.
use super::{
    Cell,
    connectivity::{self, Connectivity},
    faces::{self, Face},
    rotate34,
};
use crate::{
    mesh::{Matrix, Mesh, identity, srt_matrix},
    scene::{Handle, exec::multiply},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logic {
    pub code: u8,
    pub condition: u8,
    pub value: i32,
    pub output: u8,
    pub events: [Option<usize>; 2],
}

/// Fields assigned by AddMesh after KEvent::Init; the event scheduler creates
/// the remaining zero/default runtime state when consuming this record.
#[derive(Clone, Debug, PartialEq)]
pub struct EventSpawn {
    pub operator: usize,
    pub translate: [f32; 4],
    pub cull_distance: f32,
}

pub struct Entry {
    pub cell: Cell,
    pub sector: Option<usize>,
    pub logic: Logic,
    pub events: [Option<EventSpawn>; 2],
}

pub struct World {
    pub entries: Vec<Entry>,
    pub adds: Vec<usize>,
    pub subs: Vec<usize>,
    pub zones: Vec<usize>,
    pub connectivity: Connectivity,
    pub faces: Vec<Option<Vec<Face>>>,
}

impl World {
    /// Meshes and sector handles must come from the same initialized graph.
    pub fn build(
        root_operator: usize,
        root: &Handle,
        meshes: &BTreeMap<usize, Rc<Mesh>>,
        sectors: &[(usize, Handle)],
    ) -> Result<Self, &'static str> {
        let mut entries = Vec::new();
        collect(
            root,
            identity(),
            Some(root_operator),
            meshes,
            sectors,
            &mut BTreeSet::new(),
            &mut entries,
        )?;
        let cells: Vec<_> = entries.iter().map(|entry| entry.cell.clone()).collect();
        let connectivity = connectivity::connect(&cells);
        let mut adds = Vec::new();
        let mut subs = Vec::new();
        let mut zones = Vec::new();
        let mut faces = vec![None; entries.len()];
        for (i, entry) in entries.iter().enumerate() {
            match entry.cell.mode & 3 {
                1 => subs.push(i),
                2 => zones.push(i),
                _ => {
                    adds.push(i);
                    faces[i] = Some(faces::create(&cells, i, &connectivity.neighbors[i].adds)?);
                }
            }
        }
        Ok(Self {
            entries,
            adds,
            subs,
            zones,
            connectivity,
            faces,
        })
    }

    /// Original FindCell scans additive cells in creation order.
    pub fn find_cell(&self, point: [f32; 4]) -> Option<usize> {
        self.adds
            .iter()
            .copied()
            .find(|&i| self.entries[i].cell.outside_mask(point, 0.) == 0)
    }
}

fn collect(
    handle: &Handle,
    base: Matrix,
    mut sector: Option<usize>,
    meshes: &BTreeMap<usize, Rc<Mesh>>,
    sectors: &[(usize, Handle)],
    path: &mut BTreeSet<usize>,
    entries: &mut Vec<Entry>,
) -> Result<(), &'static str> {
    let address = Rc::as_ptr(handle) as usize;
    if !path.insert(address) {
        return Err("collision scene cycle");
    }
    let scene = handle.borrow();
    if scene.is_sector {
        sector = Some(
            sectors
                .iter()
                .find(|(_, node)| Rc::ptr_eq(node, handle))
                .map(|(id, _)| *id)
                .ok_or("collision sector has no operator identity")?,
        );
    }
    let step = srt_matrix(&scene.srt);
    let count = if scene.count == 0 { 1 } else { scene.count };
    let mut matrix = base;
    for i in 0..count {
        if scene.count == 0 || i != 0 {
            matrix = multiply(step, matrix);
        }
        for child in &scene.children {
            collect(child, matrix, sector, meshes, sectors, path, entries)?;
        }
        if let Some(operator) = scene.mesh {
            let mesh = meshes
                .get(&operator)
                .ok_or("collision mesh has not been generated")?;
            for collision in &mesh.collision {
                let mut vertices = [[0.; 4]; 8];
                for (v, &index) in vertices.iter_mut().zip(&collision.vertices) {
                    let vertex = mesh
                        .vertices
                        .get(index)
                        .ok_or("collision mesh vertex out of range")?;
                    *v = rotate34(matrix, vertex.attributes[0]);
                }
                let cell = Cell::from_vertices(vertices, collision.mode as u32);
                let events = collision.events.map(|operator| {
                    operator.map(|operator| EventSpawn {
                        operator,
                        translate: [
                            (cell.bounds[0][0] + cell.bounds[1][0]) / 2.,
                            cell.bounds[0][1],
                            (cell.bounds[0][2] + cell.bounds[1][2]) / 2.,
                            0.,
                        ],
                        cull_distance: 35.,
                    })
                });
                entries.push(Entry {
                    cell,
                    sector,
                    events,
                    logic: Logic {
                        code: collision.code,
                        condition: collision.condition,
                        value: collision.value,
                        output: collision.output,
                        events: collision.events,
                    },
                });
            }
        }
    }
    path.remove(&address);
    Ok(())
}
