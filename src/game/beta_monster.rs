//! Released 2004 monster initialization ABI. This is separate from the later
//! development actor: no walk-style, weapon-kind or spawn-switch words exist.

pub mod body;

/// Fields copied by the released handler at 0x81d82f..0x81d85e.
/// The first vector feeds InitEuler in radians; the second is added to the
/// parent translation by the released initializer.
#[derive(Clone, Debug, PartialEq)]
pub struct InitialCopy {
    pub vectors: [[f32; 3]; 2],
    /// The fourteen words stored at actor+0x40, with speed converted per ms.
    pub parameters: [u32; 14],
}
impl InitialCopy {
    pub fn decode(words: &[u32]) -> Result<Self, &'static str> {
        let words = words
            .get(..26)
            .ok_or("truncated released monster parameters")?;
        let vectors = std::array::from_fn(|v| {
            std::array::from_fn(|axis| f32::from_bits(words[v * 3 + axis]))
        });
        let mut parameters: [u32; 14] = words[6..20].try_into().unwrap();
        // FMUL uses the single-precision constant at released VA 0x81f2c4.
        // Multiplication is required; division by 1000 can round differently.
        // KOp::Exec selects x87 CW 0x143f: 24-bit precision, round down.
        let product =
            f64::from(f32::from_bits(parameters[0])) * f64::from(f32::from_bits(0x3a83126f));
        let rounded = product as f32;
        parameters[0] = if f64::from(rounded) > product {
            rounded.next_down()
        } else {
            rounded
        }
        .to_bits();
        Ok(Self {
            vectors,
            parameters,
        })
    }
}

/// Local geometry passed to the released collision-cell initializer.
#[derive(Clone, Debug, PartialEq)]
pub struct HitBox {
    pub matrix: crate::mesh::Matrix,
    pub scale: [f32; 3],
}
fn down(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest) > value {
        nearest.next_down()
    } else {
        nearest
    }
}
// x87 arithmetic keeps the wide exponent even with 24-bit precision.
fn precision24(value: f64) -> f64 {
    if value == 0. || !value.is_finite() {
        return value;
    }
    const MASK: u64 = (1 << 29) - 1;
    let bits = value.to_bits();
    let mut truncated = bits & !MASK;
    if value.is_sign_negative() && bits & MASK != 0 {
        truncated += 1 << 29;
    }
    f64::from_bits(truncated)
}
fn rounded_arithmetic(value: f64, error: f64) -> f64 {
    let rounded = precision24(value);
    if error < 0. && rounded == value && value != 0. && value.is_finite() {
        let bits = value.to_bits();
        if value > 0. {
            f64::from_bits((bits - 1) & !((1 << 29) - 1))
        } else {
            f64::from_bits(bits + (1 << 29))
        }
    } else {
        rounded
    }
}
fn mul(a: f64, b: f64) -> f64 {
    let product = a * b;
    rounded_arithmetic(product, a.mul_add(b, -product))
}
fn add(a: f64, b: f64) -> f64 {
    // Exact cancellation is negative zero under round-toward-negative.
    if a == -b && (a.is_sign_negative() || b.is_sign_negative()) {
        -0.
    } else {
        let sum = a + b;
        let z = sum - a;
        let error = (a - (sum - z)) + (b - z);
        rounded_arithmetic(sum, error)
    }
}
impl HitBox {
    pub fn decode(words: &[u32]) -> Result<Self, &'static str> {
        let p = words
            .get(..26)
            .ok_or("truncated released monster parameters")?;
        let f = |i| f32::from_bits(p[i]);
        let (sx, cx) = f64::from(f(0)).sin_cos();
        let (sy, cy) = f64::from(f(1)).sin_cos();
        let (sz, cz) = f64::from(f(2)).sin_cos();
        // InitEuler spills cy and cz to f32 but retains cz on the x87 stack.
        let cy = f64::from(down(cy));
        let stored_cz = f64::from(down(cz));
        let cz_sy = mul(cz, sy);
        let sz_sy = mul(sz, sy);
        let mut matrix = [
            [down(mul(cy, cz)), down(mul(sz, cy)), down(-sy), 0.],
            [
                down(add(mul(cz_sy, sx), -mul(sz, cx))),
                down(add(mul(sz_sy, sx), mul(stored_cz, cx))),
                down(mul(cy, sx)),
                0.,
            ],
            [
                down(add(mul(cz_sy, cx), mul(sz, sx))),
                down(add(mul(f64::from(down(sz_sy)), cx), -mul(stored_cz, sx))),
                down(mul(cy, cx)),
                0.,
            ],
            [0., 0., 0., 1.],
        ];
        for axis in 0..3 {
            let a = f64::from(down(mul(f64::from(f(23)), f64::from(matrix[0][axis]))));
            let b = mul(f64::from(f(24)), f64::from(matrix[1][axis]));
            let c = mul(f64::from(f(25)), f64::from(matrix[2][axis]));
            let ab = f64::from(down(add(b, a)));
            matrix[3][axis] = down(add(c, ab));
        }
        Ok(Self {
            matrix,
            scale: [f(20), f(21), f(22)],
        })
    }
}

impl HitBox {
    /// Vertices supplied by the released initializer to its plane builder.
    pub fn vertices(&self) -> [[f32; 4]; 8] {
        std::array::from_fn(|index| {
            let corner: [f32; 3] = std::array::from_fn(|axis| {
                down(-mul(
                    ((index >> axis) & 1) as f64 - 0.5,
                    f64::from(self.scale[axis]),
                ))
            });
            std::array::from_fn(|axis| {
                let a = down(mul(f64::from(corner[0]), f64::from(self.matrix[0][axis])));
                let b = mul(f64::from(corner[1]), f64::from(self.matrix[1][axis]));
                let ab = down(add(b, f64::from(a)));
                let c = mul(f64::from(corner[2]), f64::from(self.matrix[2][axis]));
                let abc = down(add(c, f64::from(ab)));
                down(add(f64::from(abc), f64::from(self.matrix[3][axis])))
            })
        })
    }
}

/// Released spawn fields before collider construction. Event words retain the
/// released layout; they must not be reinterpreted as development KEvent fields.
#[derive(Clone, Debug, PartialEq)]
pub struct InitialState {
    pub copied: InitialCopy,
    pub matrix: crate::mesh::Matrix,
    pub event_words: [u32; 45],
    pub death_event_words: [u32; 45],
    pub spline_token: u32,
    pub life: i32,
    pub cell_query_position: [f32; 4],
    pub collider_type: u32,
}
impl InitialState {
    /// Tokens represent released object identities, never native Rust pointers.
    pub fn decode(
        words: &[u32],
        parent: crate::mesh::Matrix,
        spline_token: u32,
        actor_token: u32,
    ) -> Result<Self, &'static str> {
        let copied = InitialCopy::decode(words)?;
        let mut event_words = [0; 45];
        // actor+0xbc and actor+0xcc, relative to Event at actor+0x78.
        for axis in 0..3 {
            event_words[17 + axis] = copied.vectors[0][axis].to_bits();
            event_words[21 + axis] = down(add(
                f64::from(copied.vectors[1][axis]),
                f64::from(parent[3][axis]),
            ))
            .to_bits();
        }
        // Init() at actor+0xe4 constructs the identity, then replaces l.xyz
        // from actor+0xcc and l.w with 1. Actor identity follows at +0x124.
        let mut matrix = crate::mesh::identity();
        for axis in 0..3 {
            matrix[3][axis] = f32::from_bits(event_words[21 + axis]);
        }
        for (out, value) in event_words[27..43]
            .iter_mut()
            .zip(matrix.into_iter().flatten())
        {
            *out = value.to_bits();
        }
        event_words[43] = actor_token;
        let life = copied.parameters[4] as i32;
        Ok(Self {
            copied,
            matrix: parent,
            event_words,
            death_event_words: [0; 45],
            spline_token,
            life,
            cell_query_position: [
                parent[3][0],
                down(add(f64::from(parent[3][1]), 0.5)),
                parent[3][2],
                parent[3][3],
            ],
            collider_type: 3,
        })
    }
}
