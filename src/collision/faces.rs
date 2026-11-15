//! Original GenSimpleFace clipping and CreateCollisionFaces recursion.
use super::{Cell, ORDER, dot};
use std::collections::VecDeque;

pub type Face = Vec<[f32; 4]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipCode {
    AllOnPlane,
    NotCrossed,
    Crossed,
}

fn classify(v: [f32; 4], plane: [f32; 4]) -> usize {
    let distance = dot(v, plane) + plane[3];
    if distance > 1e-3 {
        0
    } else if distance < -1e-3 {
        1
    } else {
        2
    }
}

pub fn clip(face: &[[f32; 4]], plane: [f32; 4]) -> ([Face; 2], ClipCode) {
    let mut sides = [Vec::new(), Vec::new()];
    let Some(&mut_previous) = face.last() else {
        return (sides, ClipCode::AllOnPlane);
    };
    let mut previous = mut_previous;
    let mut last_side = classify(previous, plane);
    let mut all_on = true;
    let mut crossed = false;
    for &v in face {
        let side = classify(v, plane);
        if side == 2 {
            sides[0].push(v);
            sides[1].push(v);
        } else if side == last_side || last_side == 2 {
            sides[side].push(v);
        } else {
            let d1 = dot(previous, plane) + plane[3];
            let d2 = dot(v, plane) + plane[3];
            let t = -d1 / (d2 - d1);
            let intersection = std::array::from_fn(|i| previous[i] + (v[i] - previous[i]) * t);
            sides[0].push(intersection);
            sides[1].push(intersection);
            sides[side].push(v);
            crossed = true;
        }
        if side != 2 {
            all_on = false;
        }
        last_side = side;
        previous = v;
    }
    (
        sides,
        if all_on {
            ClipCode::AllOnPlane
        } else if crossed {
            ClipCode::Crossed
        } else {
            ClipCode::NotCrossed
        },
    )
}

fn split(face: &Face, cell: &Cell, plane: usize, output: &mut VecDeque<Face>) -> bool {
    let count = cell.planes.len();
    if count == 0 {
        return true;
    }
    let a: [f32; 3] = std::array::from_fn(|i| face[2][i] - face[1][i]);
    let b: [f32; 3] = std::array::from_fn(|i| face[0][i] - face[1][i]);
    let normal = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
        0.,
    ];
    let (sides, _) = clip(face, cell.planes[plane]);
    let mut keep = [false; 2];
    for i in 0..2 {
        keep[i] = sides[i].len() > 2;
        if keep[i] && plane == count - 1 {
            keep[i] = false;
            let mut center = [0.; 4];
            for v in &sides[i] {
                for j in 0..3 {
                    center[j] += v[j];
                }
            }
            let scale = 1. / sides[i].len() as f32;
            for x in &mut center[..3] {
                *x *= scale;
            }
            // Source loops through j<=PlaneCount. The next array slot is zero
            // after Init, or Center immediately following Planes[12].
            let trailing = if count == 12 { cell.center } else { [0.; 4] };
            for &p in cell.planes.iter().chain(std::iter::once(&trailing)) {
                let d = dot(p, center) + p[3];
                if d < 0. || (d < 4.010e-3 * 4. && dot(p, normal) > 0.) {
                    keep[i] = true;
                }
            }
        }
        if keep[i] && plane < count - 1 {
            keep[i] = split(&sides[i], cell, plane + 1, output);
        }
    }
    if !(keep[0] && keep[1]) {
        for (side, keep) in sides.into_iter().zip(keep) {
            if keep {
                output.push_front(side);
            }
        }
    }
    keep[0] && keep[1]
}

/// Generate one add cell's faces, retaining source linked-list ordering.
pub fn create(cells: &[Cell], add: usize, neighbors: &[usize]) -> Result<Vec<Face>, &'static str> {
    let source = cells.get(add).ok_or("collision add index out of range")?;
    let mut faces = VecDeque::new();
    for order in ORDER {
        faces.push_front(order.map(|i| source.vertices[i]).to_vec());
    }
    for &neighbor in neighbors {
        let cell = cells
            .get(neighbor)
            .ok_or("collision neighbor index out of range")?;
        if cell.planes.len() > 12 {
            return Err("collision plane count exceeds source limit");
        }
        let old = std::mem::take(&mut faces);
        for face in old {
            if split(&face, cell, 0, &mut faces) {
                faces.push_front(face);
            }
        }
    }
    Ok(faces.into_iter().collect())
}
