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
