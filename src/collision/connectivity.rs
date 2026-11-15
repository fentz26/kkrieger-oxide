//! Original CellConnect bounding-box sweep, before collision-face generation.
use super::Cell;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Neighbors {
    pub adds: Vec<usize>,
    pub subs: Vec<usize>,
    pub zones: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connectivity {
    pub sorted: Vec<usize>,
    pub neighbors: Vec<Neighbors>,
}

fn sift(cells: &[Cell], sorted: &mut [usize], n: usize, mut k: usize) {
    let value = sorted[k];
    let key = cells[value].bounds[0][0];
    while k < n >> 1 {
        let mut j = k * 2 + 1;
        if j + 1 < n && cells[sorted[j]].bounds[0][0] < cells[sorted[j + 1]].bounds[0][0] {
            j += 1;
        }
        if key >= cells[sorted[j]].bounds[0][0] {
            break;
        }
        sorted[k] = sorted[j];
        k = j;
    }
    sorted[k] = value;
}

/// Preserve the source heap's equal-key ordering and adjacency insertion order.
/// Collision faces must subsequently be generated from these connections.
pub fn connect(cells: &[Cell]) -> Connectivity {
    let mut sorted: Vec<_> = (0..cells.len()).collect();
    for k in (0..cells.len() / 2).rev() {
        sift(cells, &mut sorted, cells.len(), k);
    }
    for n in (1..cells.len()).rev() {
        sorted.swap(0, n);
        sift(cells, &mut sorted, n, 0);
    }
    let mut neighbors = vec![Neighbors::default(); cells.len()];
    for (i, &a) in sorted.iter().enumerate() {
        let first = &cells[a];
        for &b in &sorted[i + 1..] {
            let second = &cells[b];
            let overlaps_x = second.bounds[0][0] <= first.bounds[1][0];
            if !overlaps_x {
                break;
            }
            let am = first.mode & 3;
            let bm = second.mode & 3;
            if am != 0 && bm != 0 {
                continue;
            }
            if first.bounds[1][2] < second.bounds[0][2]
                || first.bounds[0][2] > second.bounds[1][2]
                || first.bounds[1][1] < second.bounds[0][1]
                || first.bounds[0][1] > second.bounds[1][1]
            {
                continue;
            }
            if am == 0 && bm == 0 {
                neighbors[a].adds.push(b);
                neighbors[b].adds.push(a);
            } else {
                let (add, other, mode) = if am == 0 { (a, b, bm) } else { (b, a, am) };
                match mode {
                    1 => neighbors[add].subs.push(other),
                    2 => neighbors[add].zones.push(other),
                    _ => {}
                }
            }
        }
    }
    Connectivity { sorted, neighbors }
}
