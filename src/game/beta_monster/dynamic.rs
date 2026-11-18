//! Released dynamic-cell ApplyMatrix plane update.
use super::{add, down, mul, rounded_arithmetic};
use crate::mesh::Matrix;
#[derive(Clone, Debug)]
pub struct Pose {
    pub planes: [[f32; 4]; 6],
    pub count: usize,
    pub center: [f32; 4],
    pub matrix: Matrix,
    pub blocked: bool,
}
fn rotate(v: [f32; 3], m: Matrix) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let a = down(mul(f64::from(v[0]), f64::from(m[0][axis])));
        let b = down(add(
            mul(f64::from(v[1]), f64::from(m[1][axis])),
            f64::from(a),
        ));
        down(add(
            mul(f64::from(v[2]), f64::from(m[2][axis])),
            f64::from(b),
        ))
    })
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f64 {
    add(
        add(
            mul(f64::from(a[2]), f64::from(b[2])),
            mul(f64::from(a[1]), f64::from(b[1])),
        ),
        mul(f64::from(a[0]), f64::from(b[0])),
    )
}
impl Pose {
    pub fn apply(original: &[[f32; 4]], matrix: Matrix) -> Result<Self, &'static str> {
        if original.len() > 6 {
            return Err("dynamic plane capacity exceeded");
        }
        let mut planes = [[0.; 4]; 6];
        for (out, &plane) in planes.iter_mut().zip(original) {
            let n = [plane[0], plane[1], plane[2]];
            let point = n.map(|v| down(mul(f64::from(v), -f64::from(plane[3]))));
            let point = rotate(point, matrix);
            let point =
                std::array::from_fn(|i| down(add(f64::from(point[i]), f64::from(matrix[3][i]))));
            let n = rotate(n, matrix);
            let squared = dot(n, n);
            let root = squared.sqrt();
            let root = rounded_arithmetic(root, (-root).mul_add(root, squared));
            let q = 1. / root;
            let factor = down(rounded_arithmetic(q, (-q).mul_add(root, 1.) / root));
            let n = n.map(|v| down(mul(f64::from(v), f64::from(factor))));
            *out = [n[0], n[1], n[2], down(-dot(point, n))];
        }
        Ok(Self {
            planes,
            count: original.len(),
            center: matrix[3],
            matrix,
            blocked: false,
        })
    }
}

fn inverse_scale(v: [f32; 3]) -> f32 {
    let squared = dot(v, v);
    let root = squared.sqrt();
    let root = rounded_arithmetic(root, (-root).mul_add(root, squared));
    let q = 1. / root;
    down(rounded_arithmetic(q, (-q).mul_add(root, 1.) / root))
}
/// Exact released dynamic-motion delta, including the original inverse
/// algorithm for scaled bases. It is not a general-purpose matrix inverse.
pub fn motion_delta(mut confirmed: Matrix, current: Matrix) -> Matrix {
    let factors: [f32; 3] = std::array::from_fn(|row| {
        inverse_scale([confirmed[row][0], confirmed[row][1], confirmed[row][2]])
    });
    for (row, factor) in confirmed[..3].iter_mut().zip(factors) {
        for value in &mut row[..3] {
            *value = down(mul(f64::from(*value), f64::from(factor)));
        }
    }
    for row in 0..3 {
        for axis in row + 1..3 {
            let value = confirmed[row][axis];
            confirmed[row][axis] = confirmed[axis][row];
            confirmed[axis][row] = value;
        }
    }
    for (row, factor) in confirmed[..3].iter_mut().zip(factors) {
        for value in &mut row[..3] {
            *value = down(mul(f64::from(*value), f64::from(factor)));
        }
    }
    let translation = rotate(
        [-confirmed[3][0], -confirmed[3][1], -confirmed[3][2]],
        confirmed,
    );
    confirmed[3][..3].copy_from_slice(&translation);
    std::array::from_fn(|row| {
        std::array::from_fn(|axis| {
            let mut value = mul(f64::from(confirmed[row][3]), f64::from(current[3][axis]));
            for term in (0..3).rev() {
                value = add(
                    value,
                    mul(
                        f64::from(confirmed[row][term]),
                        f64::from(current[term][axis]),
                    ),
                );
            }
            down(value)
        })
    })
}

/// Proposed particle position for moving-cell reconciliation. The caller first
/// checks containment and ownership filters, then probes world collision in mode
/// 1 and restores Pos. None blocks the cell; it supplies no position to commit.
pub fn project_particle(point: [f32; 4], delta: Matrix, planes: &[[f32; 4]]) -> Option<[f32; 4]> {
    let moved: [f32; 3] = std::array::from_fn(|axis| {
        let a = down(mul(f64::from(point[0]), f64::from(delta[0][axis])));
        let b = down(add(
            mul(f64::from(point[1]), f64::from(delta[1][axis])),
            f64::from(a),
        ));
        let c = down(add(
            mul(f64::from(point[2]), f64::from(delta[2][axis])),
            f64::from(b),
        ));
        down(add(
            mul(f64::from(point[3]), f64::from(delta[3][axis])),
            f64::from(c),
        ))
    });
    let direction: [f32; 3] =
        std::array::from_fn(|i| down(add(f64::from(point[i]), -f64::from(moved[i]))));
    let squared = dot(direction, direction);
    if squared < 1e-16 {
        return None;
    }
    let factor = inverse_scale(direction);
    let direction = direction.map(|v| down(mul(f64::from(v), f64::from(factor))));
    let hit = super::hull::intersect_in(
        planes,
        point,
        [direction[0], direction[1], direction[2], 0.],
    )?;
    let mut proposal = point;
    for axis in 0..3 {
        proposal[axis] = down(add(
            mul(f64::from(direction[axis]), f64::from(hit.distance)),
            f64::from(proposal[axis]),
        ));
        proposal[axis] = down(add(
            mul(
                f64::from(hit.plane[axis]),
                -f64::from(super::collision::SEPARATION),
            ),
            f64::from(proposal[axis]),
        ));
    }
    proposal[3] = 1.;
    Some(proposal)
}

/// Owned pose state used by the dynamic phase. Part-box motion, when present,
/// must run before reconciliation; this type does not own a part-box registry.
#[derive(Clone, Debug)]
pub struct MovingCell {
    pub original: [[f32; 4]; 6],
    pub count: usize,
    pub confirmed: Matrix,
    pub current: Matrix,
    pub mode: u32,
    pub monster: bool,
    pub pose: Pose,
}
impl MovingCell {
    /// Probe each eligible contained particle. The bool identifies the particle's
    /// own cell, which is skipped. Probe runs in response mode 1; Pos is restored
    /// even when it reports failure, while other callback effects remain.
    pub fn reconcile<'p, I, F>(&mut self, particles: I, mut probe: F) -> Result<(), &'static str>
    where
        I: IntoIterator<Item = (&'p mut super::body::Particle, bool)>,
        F: FnMut(&mut super::body::Particle) -> Result<super::collision::Completion, &'static str>,
    {
        let original = self
            .original
            .get(..self.count)
            .ok_or("dynamic plane capacity exceeded")?;
        let proposed = Pose::apply(original, self.current)?;
        let delta = motion_delta(self.confirmed, self.current);
        let mut blocked = false;
        for (particle, owned) in particles {
            if owned || (self.monster && !(8..16).contains(&particle.flags[3])) {
                continue;
            }
            let position = particle.vectors[0];
            if super::hull::outside_mask(&proposed.planes[..self.count], position, 0.) != 0 {
                continue;
            }
            let Some(target) = project_particle(position, delta, &proposed.planes[..self.count])
            else {
                blocked = true;
                continue;
            };
            particle.vectors[0] = target;
            let result = probe(particle);
            particle.vectors[0] = position;
            match result {
                Ok(super::collision::Completion::Clear) => {}
                Ok(_) => blocked = true,
                Err(error) => {
                    self.pose.blocked = true;
                    return Err(error);
                }
            }
        }
        if blocked {
            self.pose.blocked = true;
        } else {
            self.mode &= !0x20;
            self.confirmed = self.current;
            self.pose = proposed;
        }
        Ok(())
    }
}
