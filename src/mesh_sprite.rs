//! Original EngMesh::PaintJob MPP_SPRITES geometry, without bone substitution.
use crate::{
    material::Program,
    mesh::Matrix,
    prepared_mesh::{Job, PreparedMesh},
};
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
/// The original uses unnormalized transposed MODELVIEW axes and pass Size;
/// its local Aspect variable is unused. GeoBegin splits at 4096 source vertices.
pub fn batches(
    mesh: &PreparedMesh,
    job: &Job,
    size: f32,
    model_view: Matrix,
) -> Result<Vec<Vec<Vertex>>, &'static str> {
    if job.program != Program::Sprites {
        return Err("sprite expansion requires original sprite job");
    }
    let vx: [f32; 3] = std::array::from_fn(|axis| model_view[axis][0] * size);
    let vy: [f32; 3] = std::array::from_fn(|axis| model_view[axis][1] * size);
    let mut batches = Vec::new();
    for indices in job.vertices.chunks(4096) {
        let mut vertices = Vec::with_capacity(indices.len() * 4);
        for &index in indices {
            let position = mesh
                .vertices
                .get(index)
                .ok_or("sprite source vertex out of range")?
                .position;
            for (corner, uv) in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
                .into_iter()
                .enumerate()
            {
                vertices.push(Vertex {
                    position: std::array::from_fn(|axis| match corner {
                        0 => position[axis] - vx[axis] + vy[axis],
                        1 => position[axis] + vx[axis] + vy[axis],
                        2 => position[axis] + vx[axis] - vy[axis],
                        _ => position[axis] - vx[axis] - vy[axis],
                    }),
                    normal: [0., 0., 1.],
                    uv,
                });
            }
        }
        batches.push(vertices);
    }
    Ok(batches)
}
