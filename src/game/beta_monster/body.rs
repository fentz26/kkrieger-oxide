//! Particle and constraint construction for the released collider.
//! Particle links are indices; dynamic collision-cell construction is separate.
use super::{add, down, mul, precision24};

#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    /// Seven leading state words of the released particle record.
    pub flags: [u32; 7],
    pub cell: Option<usize>,
    pub weight: f32,
    /// Released vectors at offsets 0x40, 0x50, 0x60 and 0x70.
    pub vectors: [[f32; 4]; 4],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Constraint {
    pub flags: [u32; 2],
    pub particles: [usize; 2],
    pub rest_length: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub particles: [Particle; 5],
    pub constraints: [Constraint; 10],
}
const DIRECTIONS: [[f64; 3]; 5] = [
    [0., 0., 0.],
    [-1., 0., 0.],
    [0., 0., -1.],
    [1., 0., 0.],
    [0., 0., 1.],
];
const EDGES: [[usize; 2]; 10] = [
    [0, 1],
    [0, 2],
    [0, 3],
    [0, 4],
    [1, 2],
    [2, 3],
    [3, 4],
    [4, 1],
    [1, 3],
    [2, 4],
];
impl Body {
    pub fn new(position: [f32; 4], cell: Option<usize>, collider_type: i32) -> Self {
        let particles = std::array::from_fn(|index| {
            let mut point = [0.; 4];
            let mut delta = [0.; 4];
            for axis in 0..3 {
                let offset = down(mul(DIRECTIONS[index][axis], [0.5, 0.25, 0.5][axis]));
                point[axis] = down(add(f64::from(offset), f64::from(position[axis])));
                delta[axis] = down(add(f64::from(point[axis]), -0.));
            }
            let mut flags = [0; 7];
            flags[0] = u32::from(index == 4);
            flags[3] = collider_type.wrapping_add(i32::from(index != 0)) as u32;
            flags[5] = 1;
            Particle {
                flags,
                cell,
                weight: 1.,
                vectors: [point, position, delta, point],
            }
        });
        let constraints = std::array::from_fn(|index| {
            let [a, b] = EDGES[index];
            let d: [f64; 3] = std::array::from_fn(|axis| {
                f64::from(down(add(
                    f64::from(particles[a].vectors[0][axis]),
                    -f64::from(particles[b].vectors[0][axis]),
                )))
            });
            let squared = add(add(mul(d[2], d[2]), mul(d[1], d[1])), mul(d[0], d[0]));
            Constraint {
                flags: [u32::from(index == 9), 0],
                particles: [a, b],
                rest_length: down(precision24(squared.sqrt())),
            }
        });
        Self {
            particles,
            constraints,
        }
    }
}
