//! Original wrapping integer generator and Perlin permutation, translated to Rust.
//! Farbrausch BSD-2-Clause: third_party/farbrausch-LICENSE.txt.
pub struct Random {
    state: u32,
}
impl Random {
    pub fn new(seed: i32) -> Self {
        let mixed = seed
            .wrapping_add(seed.wrapping_mul(17))
            .wrapping_add(seed.wrapping_mul(121))
            .wrapping_add(seed.wrapping_mul(121) / 17) as u32;
        let mut rng = Self { state: mixed };
        for i in 0..4 {
            rng.next_u32();
            if i < 3 {
                rng.state ^= mixed;
            }
        }
        rng
    }
    pub fn next_u32(&mut self) -> u32 {
        let first = self.state.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        let second = first.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        self.state = second;
        ((second >> 10) & 0xffff) | ((first << 6) & 0xffff0000)
    }
    pub fn below(&mut self, max: u32) -> u32 {
        self.next_u32() % max
    }
    pub fn float(&mut self) -> f32 {
        (self.next_u32() & 0x3fff_ffff) as f32 / 0x4000_0000_u32 as f32
    }
}

pub fn permutation() -> [u8; 256] {
    let mut rng = Random::new(1);
    make_permutation(&mut rng)
}
fn make_permutation(rng: &mut Random) -> [u8; 256] {
    let mut values = [0_u32; 256];
    let mut table = [0_u8; 256];
    for i in 0..256 {
        values[i] = rng.below(65536);
        table[i] = i as u8;
    }
    // Preserve the exact original swap order, including ties.
    for i in 0..255 {
        for j in i + 1..256 {
            if values[i] > values[j] {
                values.swap(i, j);
                table.swap(i, j);
            }
        }
    }
    table
}

pub struct Noise {
    permutation: [u8; 512],
    gradients: [[f32; 2]; 256],
}
impl Default for Noise {
    fn default() -> Self {
        Self::new()
    }
}
impl Noise {
    /// Original mesh vector noise, using its directed single-precision arithmetic.
    pub fn vector3_directed(&self, point: [f32; 3]) -> [f32; 3] {
        use crate::rounding::{add_down as add, mul_down as mul, sub_down as sub};
        const GRADIENTS: [[f32; 3]; 16] = [
            [1., 1., 0.],
            [-1., 1., 0.],
            [1., -1., 0.],
            [-1., -1., 0.],
            [1., 0., 1.],
            [-1., 0., 1.],
            [1., 0., -1.],
            [-1., 0., -1.],
            [0., 1., 1.],
            [0., -1., 1.],
            [0., 1., -1.],
            [0., -1., -1.],
            [1., 1., 0.],
            [-1., 1., 0.],
            [0., -1., 1.],
            [0., -1., -1.],
        ];
        let mut integer = [0; 3];
        let mut fraction = [0.; 3];
        for axis in 0..3 {
            let grid = point[axis].floor() as i32;
            let f = sub(point[axis], grid as f32);
            integer[axis] = (grid & 255) as usize;
            fraction[axis] = mul(mul(mul(f, f), f), add(10., mul(f, sub(mul(6., f), 15.))));
        }
        let [x, y, z] = integer;
        let [fx, fy, fz] = fraction;
        let gradient = |x: usize, y: usize, z: usize| {
            GRADIENTS[(self.permutation
                [self.permutation[self.permutation[x] as usize + y] as usize + z]
                & 15) as usize]
        };
        let interpolate = |a: [f32; 3], b: [f32; 3], t: f32| {
            std::array::from_fn(|i| add(a[i], mul(sub(b[i], a[i]), t)))
        };
        let mut a = interpolate(gradient(x, y, z), gradient(x + 1, y, z), fx);
        let mut b = interpolate(gradient(x, y + 1, z), gradient(x + 1, y + 1, z), fx);
        a = interpolate(a, b, fy);
        b = interpolate(gradient(x, y, z + 1), gradient(x + 1, y, z + 1), fx);
        let c = interpolate(gradient(x, y + 1, z + 1), gradient(x + 1, y + 1, z + 1), fx);
        b = interpolate(b, c, fy);
        interpolate(a, b, fz)
    }
    pub fn new() -> Self {
        let mut rng = Random::new(1);
        let p = make_permutation(&mut rng);
        let permutation = std::array::from_fn(|i| p[i & 255]);
        let mut gradients = [[0.; 2]; 256];
        for value in &mut gradients {
            loop {
                let x = rng.below(65536) as i32 - 32768;
                let y = rng.below(65536) as i32 - 32768;
                if x.wrapping_mul(x).wrapping_add(y.wrapping_mul(y)) < 32768 * 32768 {
                    *value = [x as f32 / 32768., y as f32 / 32768.];
                    break;
                }
            }
        }
        Self {
            permutation,
            gradients,
        }
    }
    pub fn sample2d(&self, x: i32, y: i32, mask: i32, seed: u8) -> f32 {
        let mask = (mask & 255) as usize;
        let vx = ((x >> 16) as usize) & mask;
        let vy = ((y >> 16) as usize) & mask;
        let mut tx = (x & 65535) as f32 / 65536.;
        let mut ty = (y & 65535) as f32 / 65536.;
        let at = |x: usize, y: usize| {
            self.gradients
                [self.permutation[x + self.permutation[y ^ seed as usize] as usize] as usize]
        };
        let g00 = at(vx, vy);
        let g01 = at((vx + 1) & mask, vy);
        let g10 = at(vx, (vy + 1) & mask);
        let g11 = at((vx + 1) & mask, (vy + 1) & mask);
        let f00 = g00[0] * tx + g00[1] * ty;
        let f01 = g01[0] * (tx - 1.) + g01[1] * ty;
        let f10 = g10[0] * tx + g10[1] * (ty - 1.);
        let f11 = g11[0] * (tx - 1.) + g11[1] * (ty - 1.);
        tx = tx * tx * tx * (10. + tx * (6. * tx - 15.));
        ty = ty * ty * ty * (10. + ty * (6. * ty - 15.));
        let a = f00 + (f01 - f00) * tx;
        let b = f10 + (f11 - f10) * tx;
        a + (b - a) * ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeded_sequence_matches_original_integer_generator() {
        let mut rng = Random::new(1);
        assert_eq!(
            std::array::from_fn::<_, 8, _>(|_| rng.next_u32()),
            [
                0x3b28a472, 0xa0bb5f6a, 0xf4bcfca8, 0xd93d8278, 0x75d7ca8b, 0x4bf62522, 0xed5d556e,
                0xa6f2b141
            ]
        );
        let mut table = permutation();
        table.sort_unstable();
        assert_eq!(table, std::array::from_fn(|i| i as u8));
    }

    #[test]
    fn noise_matches_original_cpp_for_signed_wrapped_coordinates() {
        let bytes = include_bytes!("../tests/fixtures/noise.bin");
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        assert_eq!(n, 1000);
        assert_eq!(bytes.len(), 4 + n as usize * 20);
        let noise = Noise::new();
        for (i, row) in bytes[4..].as_chunks::<20>().0.iter().enumerate() {
            let word = |i: usize| u32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap());
            assert_eq!(
                noise
                    .sample2d(
                        word(0) as i32,
                        word(1) as i32,
                        word(2) as i32,
                        word(3) as u8
                    )
                    .to_bits(),
                word(4),
                "sample {i}"
            );
        }
    }
}
