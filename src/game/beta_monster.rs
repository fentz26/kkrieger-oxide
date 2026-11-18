//! Released 2004 monster initialization ABI. This is separate from the later
//! development actor: no walk-style, weapon-kind or spawn-switch words exist.

/// Fields copied by the released handler at 0x81d82f..0x81d85e.
/// Vector meanings beyond their observed storage offsets remain unassigned.
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
