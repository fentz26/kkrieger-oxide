//! Original non-intro MPP_THICKLINES finalizer; positions remain model-local.
use crate::{
    material::Program,
    mesh::Matrix,
    mesh_sprite::Vertex,
    prepared_mesh::{Job, PreparedMesh},
};
pub fn batches(
    mesh: &PreparedMesh,
    job: &Job,
    size: f32,
    aspect: f32,
    model_view: Matrix,
) -> Result<Vec<Vec<Vertex>>, &'static str> {
    if job.program != Program::ThickLines || !job.vertices.len().is_multiple_of(2) {
        return Err("thick-line expansion requires paired original line vertices");
    }
    let s0 = size * aspect;
    let s1 = size / aspect;
    let mut batches = Vec::new();
    let transform = |v: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|a| {
            model_view[0][a] * v[0]
                + model_view[1][a] * v[1]
                + model_view[2][a] * v[2]
                + model_view[3][a]
        })
    };
    for indices in job.vertices.chunks(8192) {
        let mut vertices = Vec::with_capacity(indices.len() * 2);
        for pair in indices.as_chunks::<2>().0 {
            let v0 = mesh
                .vertices
                .get(pair[0])
                .ok_or("line source vertex out of range")?
                .position;
            let v1 = mesh
                .vertices
                .get(pair[1])
                .ok_or("line source vertex out of range")?
                .position;
            let p0 = transform(v0);
            let p1 = transform(v1);
            let ip0z = 1. / if p0[2] > 0.01 { p0[2] } else { 0.01 };
            let ip1z = 1. / if p1[2] > 0.01 { p1[2] } else { 0.01 };
            let mut pdx = p0[0] * ip0z - p1[0] * ip1z;
            let mut pdy = p0[1] * ip0z - p1[1] * ip1z;
            let ilen = (1. / f64::from(pdx * pdx + pdy * pdy).sqrt()) as f32;
            pdx *= ilen;
            pdy *= ilen;
            let vx: [f32; 3] =
                std::array::from_fn(|a| model_view[a][1] * pdx * s0 - model_view[a][0] * pdy * s0);
            let vy: [f32; 3] =
                std::array::from_fn(|a| -model_view[a][0] * pdx * s1 - model_view[a][1] * pdy * s1);
            for (corner, uv) in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
                .into_iter()
                .enumerate()
            {
                vertices.push(Vertex {
                    position: std::array::from_fn(|a| match corner {
                        0 => v1[a] - vx[a] + vy[a],
                        1 => v1[a] + vx[a] + vy[a],
                        2 => v0[a] + vx[a] - vy[a],
                        _ => v0[a] - vx[a] - vy[a],
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
