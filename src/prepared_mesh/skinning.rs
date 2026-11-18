//! Published EngMesh::EvalBones vertex deformation and tangent-frame cleanup.
use super::Vertex;
use crate::mesh::Matrix;
#[derive(Clone, Copy, Debug, Default)]
pub struct Influence {
    pub count: u8,
    pub weights: [f32; 4],
    pub matrices: [u16; 4],
}
/// Deform into caller-owned storage. Check every binding before modifying the
/// output. Weights retain source order and are deliberately not normalized.
pub fn deform(
    source: &[Vertex],
    influences: &[Influence],
    matrices: &[Matrix],
    output: &mut [Vertex],
) -> Result<(), &'static str> {
    if source.len() != influences.len() || output.len() != source.len() {
        return Err("bone vertex storage mismatch");
    }
    for influence in influences {
        if influence.count > 4 {
            return Err("bone influence count exceeds four");
        }
        if influence.matrices[..usize::from(influence.count)]
            .iter()
            .any(|&i| usize::from(i) >= matrices.len())
        {
            return Err("bone matrix index out of range");
        }
    }
    for ((input, influence), output) in source.iter().zip(influences).zip(output) {
        let mut position = [0.0; 3];
        let mut normal = [0.0; 3];
        let mut tangent = [0.0; 3];
        for bone in 0..usize::from(influence.count) {
            let weight = influence.weights[bone];
            let matrix = &matrices[usize::from(influence.matrices[bone])];
            for axis in 0..3 {
                position[axis] += weight
                    * (matrix[0][axis] * input.position[0]
                        + matrix[1][axis] * input.position[1]
                        + matrix[2][axis] * input.position[2]
                        + matrix[3][axis]);
                normal[axis] += weight
                    * (matrix[0][axis] * input.normal[0]
                        + matrix[1][axis] * input.normal[1]
                        + matrix[2][axis] * input.normal[2]);
                tangent[axis] += weight
                    * (matrix[0][axis] * input.tangent[0]
                        + matrix[1][axis] * input.tangent[1]
                        + matrix[2][axis] * input.tangent[2]);
            }
        }
        normalize(&mut normal);
        let dot = tangent[0] * normal[0] + tangent[1] * normal[1] + tangent[2] * normal[2];
        for axis in 0..3 {
            tangent[axis] -= dot * normal[axis];
        }
        normalize(&mut tangent);
        *output = Vertex {
            position,
            normal,
            tangent,
            color: input.color,
            uv: input.uv,
        };
    }
    Ok(())
}
fn normalize(vector: &mut [f32; 3]) {
    let length = vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2];
    if length != 0.0 {
        // sFSqrt takes/returns double; assignment to len rounds back to f32.
        let scale = (1.0 / f64::from(length).sqrt()) as f32;
        for value in vector {
            *value *= scale;
        }
    }
}
