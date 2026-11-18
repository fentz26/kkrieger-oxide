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
        self.reconcile_with(
            particles
                .into_iter()
                .map(|(particle, owned)| (particle, owned, ())),
            |(), particle| probe(particle),
        )
    }
    /// Reconcile with caller-owned per-particle identity/context. No temporary
    /// collection is needed when traversing registered body arrays.
    pub fn reconcile_with<'p, C, I, F>(
        &mut self,
        particles: I,
        mut probe: F,
    ) -> Result<(), &'static str>
    where
        I: IntoIterator<Item = (&'p mut super::body::Particle, bool, C)>,
        F: FnMut(
            C,
            &mut super::body::Particle,
        ) -> Result<super::collision::Completion, &'static str>,
    {
        let original = self
            .original
            .get(..self.count)
            .ok_or("dynamic plane capacity exceeded")?;
        let proposed = Pose::apply(original, self.current)?;
        let delta = motion_delta(self.confirmed, self.current);
        let mut blocked = false;
        for (particle, owned, context) in particles {
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
            let result = probe(context, particle);
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

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let factor = inverse_scale(v);
    v.map(|value| down(mul(f64::from(value), f64::from(factor))))
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        down(add(
            mul(f64::from(a[j]), f64::from(b[k])),
            -mul(f64::from(a[k]), f64::from(b[j])),
        ))
    })
}
/// Original dynamic-frame Fade: blend i/j, rebuild k and i with cross products,
/// normalize all three axes, then blend translation. Factor is not clamped.
pub fn interpolate(first: Matrix, second: Matrix, factor: f32) -> Matrix {
    let blend = |row: usize| -> [f32; 3] {
        std::array::from_fn(|axis| {
            down(add(
                mul(
                    add(f64::from(second[row][axis]), -f64::from(first[row][axis])),
                    f64::from(factor),
                ),
                f64::from(first[row][axis]),
            ))
        })
    };
    let i = blend(0);
    let j = blend(1);
    let k = normalize(cross(i, j));
    let i = normalize(cross(j, k));
    let j = normalize(j);
    let l = blend(3);
    [
        [i[0], i[1], i[2], 0.],
        [j[0], j[1], j[2], 0.],
        [k[0], k[1], k[2], 0.],
        [l[0], l[1], l[2], 1.],
    ]
}
impl MovingCell {
    /// Complete interpolation/reconciliation sequence for a dynamic frame.
    /// Publishing accepted planes and part-box handling remain caller-owned.
    pub fn reconcile_frame<'p, I, F>(
        &mut self,
        first: Matrix,
        second: Matrix,
        factor: f32,
        particles: I,
        probe: F,
    ) -> Result<(), &'static str>
    where
        I: IntoIterator<Item = (&'p mut super::body::Particle, bool)>,
        F: FnMut(&mut super::body::Particle) -> Result<super::collision::Completion, &'static str>,
    {
        self.current = interpolate(first, second, factor);
        self.reconcile(particles, probe)
    }
}

/// Registry metadata retained while geometric views are rebuilt after motion.
#[derive(Clone, Copy, Debug)]
pub struct CollisionProperties {
    pub only_for_shot: bool,
    pub player: bool,
    pub part_box: Option<usize>,
    pub collided: u32,
}
impl MovingCell {
    /// Borrow the accepted hull for a cast. Drop views before reconciliation and
    /// rebuild afterward; blocked proposals never replace the accepted planes.
    /// The caller copies updated collided masks back into registry metadata.
    pub fn collision_view(
        &self,
        properties: CollisionProperties,
    ) -> Result<super::subtractive::DynamicCell<'_>, &'static str> {
        let planes = self
            .pose
            .planes
            .get(..self.pose.count)
            .ok_or("dynamic plane capacity exceeded")?;
        Ok(super::subtractive::DynamicCell {
            planes,
            mode: self.mode,
            only_for_shot: properties.only_for_shot,
            monster: self.monster,
            player: properties.player,
            part_box: properties.part_box,
            collided: properties.collided,
        })
    }
}
