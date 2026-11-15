//! Original sector/portal visibility, including empty-box traversal and door cost.
use crate::mesh::Matrix;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect(pub [f32; 4]);
impl Rect {
    pub const UNIT: Self = Self([-1., -1., 1., 1.]);
    pub const EMPTY: Self = Self([1., 1., -1., -1.]);
    fn union(&mut self, r: Self) {
        for i in 0..2 {
            if r.0[i] < self.0[i] {
                self.0[i] = r.0[i];
            }
            if r.0[i + 2] > self.0[i + 2] {
                self.0[i + 2] = r.0[i + 2];
            }
        }
    }
    fn intersect(&mut self, r: Self) {
        for i in 0..2 {
            if r.0[i] > self.0[i] {
                self.0[i] = r.0[i];
            }
            if r.0[i + 2] < self.0[i + 2] {
                self.0[i + 2] = r.0[i + 2];
            }
        }
    }
    pub fn paints(self) -> bool {
        self.0[2] > self.0[0] && self.0[3] > self.0[1]
    }
    fn include(&mut self, v: [f32; 4]) {
        let inverse = 1. / v[3];
        let x = v[0] * inverse;
        let y = v[1] * inverse;
        // Original sMin/sMax are conditional expressions, including NaN behavior.
        self.0[0] = if self.0[0] < x { self.0[0] } else { x };
        self.0[1] = if self.0[1] < y { self.0[1] } else { y };
        self.0[2] = if self.0[2] > x { self.0[2] } else { x };
        self.0[3] = if self.0[3] > y { self.0[3] } else { y };
    }
}
#[derive(Clone, Debug)]
pub struct Portal {
    pub sectors: [usize; 2],
    pub between: Option<usize>,
    pub cost: i32,
    pub cube: [[f32; 4]; 8],
}
impl Portal {
    pub fn new(
        sectors: [usize; 2],
        between: Option<usize>,
        bounds: [[f32; 3]; 2],
        matrix: Matrix,
        cost: i32,
    ) -> Self {
        let cube = std::array::from_fn(|i| {
            let p: [f32; 3] =
                std::array::from_fn(|axis| bounds[usize::from(i & (1 << axis) != 0)][axis]);
            std::array::from_fn(|axis| {
                matrix[0][axis] * p[0]
                    + matrix[1][axis] * p[1]
                    + matrix[2][axis] * p[2]
                    + matrix[3][axis]
            })
        });
        Self {
            sectors,
            between,
            cost,
            cube,
        }
    }
    fn projected_box(&self, projection: Matrix, parent: Rect) -> Rect {
        const EDGES: [[usize; 2]; 12] = [
            [0, 1],
            [1, 5],
            [5, 4],
            [4, 0],
            [2, 3],
            [3, 7],
            [7, 6],
            [6, 2],
            [0, 2],
            [1, 3],
            [5, 7],
            [4, 6],
        ];
        let transformed: [[f32; 4]; 8] = self.cube.map(|p| {
            std::array::from_fn(|a| {
                projection[0][a] * p[0]
                    + projection[1][a] * p[1]
                    + projection[2][a] * p[2]
                    + projection[3][a] * p[3]
            })
        });
        let mut bounds = Rect::EMPTY;
        let mut clip = false;
        for p in transformed {
            if p[2] >= 0. {
                bounds.include(p);
            } else {
                clip = true;
            }
        }
        if clip {
            let mut found = false;
            for [a, b] in EDGES {
                let a = transformed[a];
                let b = transformed[b];
                if a[2] * b[2] < 0. {
                    let t = a[2] / (a[2] - b[2]);
                    bounds.include(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t));
                    found = true;
                }
            }
            if !found {
                bounds = Rect::EMPTY;
            }
        }
        bounds.intersect(parent);
        if bounds.0[2] - bounds.0[0] <= 0. || bounds.0[3] - bounds.0[1] <= 0. {
            bounds = Rect::EMPTY;
        }
        bounds
    }
}
#[derive(Clone, Debug)]
pub struct Sector {
    pub execute: bool,
    pub bounds: Rect,
    visited: u32,
}
/// Portals are supplied in creation order. The source scans their linked list
/// newest-first; between-sector propagation deliberately uses that same order.
pub fn compute(
    count: usize,
    portals: &[Portal],
    observer: Option<usize>,
    projection: Matrix,
) -> Result<Vec<Sector>, &'static str> {
    if observer.is_some_and(|i| i >= count)
        || portals.iter().any(|p| {
            p.sectors
                .iter()
                .chain(p.between.iter())
                .any(|&i| i >= count)
        })
    {
        return Err("portal sector index out of range");
    }
    let mut sectors = vec![
        Sector {
            execute: false,
            bounds: Rect([1e15, 1e15, -1e15, -1e15]),
            visited: 0
        };
        count
    ];
    if let Some(observer) = observer {
        visit(&mut sectors, portals, projection, observer, 0, Rect::UNIT);
        for p in portals.iter().rev() {
            if let Some(i) = p.between
                && (sectors[p.sectors[0]].execute || sectors[p.sectors[1]].execute)
            {
                sectors[i].execute = true;
                sectors[i].bounds = Rect::UNIT;
            }
        }
    } else {
        for sector in &mut sectors {
            sector.execute = true;
            sector.bounds = Rect::UNIT;
        }
    }
    Ok(sectors)
}
/// Repeated views in one frame retain each scene's accumulated portal bounds.
/// Sector paint flags are reset by the execution phase between these calls.
pub fn compute_with_history(
    count: usize,
    portals: &[Portal],
    observer: Option<usize>,
    projection: Matrix,
    previous: &[Rect],
) -> Result<Vec<Sector>, &'static str> {
    if !previous.is_empty() && previous.len() != count {
        return Err("sector visibility history count changed");
    }
    let mut sectors = compute(count, portals, observer, projection)?;
    for (sector, &old) in sectors.iter_mut().zip(previous) {
        let mut bounds = old;
        bounds.union(sector.bounds);
        sector.bounds = bounds;
    }
    Ok(sectors)
}
fn visit(
    sectors: &mut [Sector],
    portals: &[Portal],
    projection: Matrix,
    start: usize,
    cost: i32,
    bounds: Rect,
) {
    sectors[start].visited += 1;
    sectors[start].execute = true;
    sectors[start].bounds.union(bounds);
    for p in portals.iter().rev() {
        let next_cost = cost.wrapping_add(p.cost);
        if next_cost < 256 {
            for side in 0..2 {
                let next = p.sectors[side ^ 1];
                if p.sectors[side] == start && sectors[next].visited == 0 {
                    visit(
                        sectors,
                        portals,
                        projection,
                        next,
                        next_cost,
                        p.projected_box(projection, bounds),
                    );
                }
            }
        }
    }
    sectors[start].visited -= 1;
}
