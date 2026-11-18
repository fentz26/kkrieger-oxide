//! Published GenMinMeshAnim::EvalAnimation local tracks and skeleton hierarchy.
use crate::mesh::Matrix;
pub enum Local<'a> {
    Static(Matrix),
    Keys {
        srt: [f32; 9],
        count: usize,
        tracks: [Option<&'a [[f32; 3]]>; 3],
    },
    Spline {
        factor: f32,
        offset: f32,
    },
}
pub struct Node<'a> {
    pub parent: Option<usize>,
    pub base_pose: Matrix,
    pub local: Local<'a>,
}
/// Temporary forward-kinematic matrices persist between calls, as in the
/// source. A forward parent reference therefore reads its preceding value.
/// Bind-pose output is separate from those temporary matrices. Spline callers
/// receive both adjusted time and clamped original time, in original order.
pub fn evaluate(
    nodes: &[Node<'_>],
    time: f32,
    temporary: &mut [Matrix],
    output: &mut [Matrix],
    mut spline: impl FnMut(usize, f32, f32) -> Result<Matrix, &'static str>,
) -> Result<(), &'static str> {
    if !time.is_finite() {
        return Err("non-finite skeleton time");
    }
    if temporary.len() != nodes.len() || output.len() != nodes.len() {
        return Err("skeleton matrix storage mismatch");
    }
    let time = if time < 0.0 { 0.0 } else { time };
    // Validate all indexes and track lengths before changing any matrix.
    for node in nodes {
        if node.parent.is_some_and(|p| p >= nodes.len()) {
            return Err("skeleton parent index out of range");
        }
        match &node.local {
            Local::Keys { count, tracks, .. } => {
                if *count == 0
                    || *count > i32::MAX as usize
                    || tracks.iter().flatten().any(|track| track.len() != *count)
                {
                    return Err("invalid skeleton key storage");
                }
                let frame = (*count as f32 * time) * 1024.0;
                if !frame.is_finite() || frame >= 2147483648.0 {
                    return Err("skeleton key time exceeds signed frame range");
                }
            }
            Local::Spline { factor, offset }
                if !factor.is_finite()
                    || !offset.is_finite()
                    || !(time * factor + offset).is_finite() =>
            {
                return Err("invalid skeleton spline time");
            }
            _ => {}
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        let local = match &node.local {
            Local::Static(matrix) => *matrix,
            Local::Spline { factor, offset } => spline(index, time * factor + offset, time)?,
            Local::Keys { srt, count, tracks } => {
                let frame = (*count as f32 * time * 1024.0) as i32;
                let fraction = (frame & 1023) as f32 * (1.0 / 1024.0);
                let k0 = (frame as usize / 1024) % count;
                let k1 = (k0 + 1) % count;
                let mut srt = *srt;
                for (component, track) in tracks.iter().enumerate() {
                    if let Some(track) = track {
                        for axis in 0..3 {
                            let a = track[k0][axis];
                            let b = track[k1][axis];
                            srt[component * 3 + axis] = a + (b - a) * fraction;
                        }
                    }
                }
                srt_matrix(&srt)
            }
        };
        temporary[index] = match node.parent {
            Some(parent) => affine(local, temporary[parent]),
            None => local,
        };
        output[index] = affine(node.base_pose, temporary[index]);
    }
    Ok(())
}
// Original sMatrix::MulA: three-term rotational products and a separate
// translation add, with explicit homogeneous components.
fn affine(a: Matrix, b: Matrix) -> Matrix {
    let mut result = std::array::from_fn(|row| {
        let mut vector = [0.0; 4];
        for axis in 0..3 {
            vector[axis] = b[0][axis] * a[row][0] + b[1][axis] * a[row][1] + b[2][axis] * a[row][2];
        }
        vector
    });
    for axis in 0..3 {
        result[3][axis] += b[3][axis];
    }
    result[3][3] = 1.0;
    result
}

fn srt_matrix(srt: &[f32; 9]) -> Matrix {
    let mut matrix = if srt[3..6].iter().all(|v| v.abs() < 1e-6) {
        crate::mesh::identity()
    } else {
        crate::mesh::euler_matrix_fsincos(
            [srt[3], srt[4], srt[5]].map(|angle| angle * std::f32::consts::TAU),
        )
    };
    for (column, scale) in matrix[..3].iter_mut().zip(srt) {
        for value in &mut column[..3] {
            *value *= scale;
        }
    }
    matrix[3] = [srt[6], srt[7], srt[8], 1.0];
    matrix
}
