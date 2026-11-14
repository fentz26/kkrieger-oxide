//! Original UpdateShadowCacheJob classification, silhouette sides and caps.
use crate::{
    material::Program,
    prepared_mesh::{Job, PreparedMesh},
};
#[derive(Clone, Debug)]
pub struct Volume {
    pub vertices: Vec<[f32; 4]>,
    pub indices: Vec<usize>,
    pub silhouette_indices: usize,
    pub cap_indices: usize,
}
impl Volume {
    pub fn draw_indices(&self, zfail: bool) -> &[usize] {
        &self.indices[..self.silhouette_indices + if zfail { self.cap_indices } else { 0 }]
    }
}
/// The source uses one zero-initialized global classification table and retains
/// entries beyond the current job's planes. Keep that behavior across jobs.
pub struct Context {
    face_in: Vec<u8>,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            face_in: vec![0; 65536],
        }
    }
}
impl Context {
    pub fn build(
        &mut self,
        mesh: &PreparedMesh,
        job: &Job,
        light: [f32; 3],
    ) -> Result<Volume, &'static str> {
        if job.program != Program::Shadow {
            return Err("shadow volume requires an original shadow job");
        }
        if job.planes.len() > self.face_in.len() || mesh.part_bounds.len() >= 16384 {
            return Err("original shadow classification limits exceeded");
        }
        for (i, p) in job.planes.iter().enumerate() {
            let d = p[0] * light[0] + p[1] * light[1] + p[2] * light[2] + p[3];
            self.face_in[i] = u8::from(d >= 0.);
        }
        let mut indices = Vec::with_capacity(job.edges.len() * 6 + job.indices.len());
        for edge in &job.edges {
            let first = *self
                .face_in
                .get(edge.faces[0] as usize)
                .ok_or("shadow face index out of range")?;
            let second = *self
                .face_in
                .get(edge.faces[1] as usize)
                .ok_or("shadow face index out of range")?;
            if first != second {
                let k = first as usize;
                let i0 = edge.vertices[k ^ 1] as usize;
                let i1 = edge.vertices[k] as usize;
                indices.extend([i0, i1, i1 + 1, i0, i1 + 1, i0 + 1]);
            }
        }
        let silhouette_indices = indices.len();
        if job.face_map.len() * 3 != job.indices.len() {
            return Err("invalid shadow cap mapping");
        }
        for (&face, triangle) in job.face_map.iter().zip(job.indices.as_chunks::<3>().0) {
            let offset = *self
                .face_in
                .get(face)
                .ok_or("shadow cap face index out of range")? as usize;
            indices.extend(triangle.iter().map(|&index| index + offset));
        }
        let mut vertices = Vec::with_capacity(job.vertices.len() * 2);
        for &i in &job.vertices {
            let p = mesh
                .vertices
                .get(i)
                .ok_or("shadow vertex index out of range")?
                .position;
            vertices.push([p[0], p[1], p[2], 1.]);
            vertices.push([p[0], p[1], p[2], 0.]);
        }
        Ok(Volume {
            vertices,
            indices,
            silhouette_indices,
            cap_indices: job.indices.len(),
        })
    }
}
