//! Original EffectPrintInfo glyph metrics and TSPACE3 geometry.
use crate::bitmap::Letter;

pub struct Layout {
    pub flags: u32,
    pub size: [f32; 2],
    pub space: [f32; 2],
    pub radius: f32,
}
#[derive(Default)]
pub struct Geometry {
    pub vertices: Vec<[f32; 8]>,
    pub indices: Vec<u16>,
}
impl Layout {
    fn character_width(&self, byte: u8, letters: &[Letter; 256]) -> f32 {
        let letter = letters[byte as usize];
        let height = letter.uv[3] - letter.uv[1];
        if height > 0. {
            letter.width / height * self.size[0]
        } else {
            0.
        }
    }
    pub fn width(&self, text: &[u8], count: Option<usize>, letters: &[Letter; 256]) -> f32 {
        let mut width = 0.;
        for &byte in text
            .iter()
            .take(count.unwrap_or(usize::MAX))
            .take_while(|&&b| b != 0)
        {
            width += self.character_width(byte, letters) + self.space[0];
        }
        width - self.space[0]
    }
    /// Append one already-expanded line. The Print formatter supplies the
    /// optional explicit center marker and advances y for authored newlines.
    pub fn append_line(
        &self,
        text: &[u8],
        center: Option<usize>,
        y: f32,
        letters: &[Letter; 256],
        geometry: &mut Geometry,
    ) {
        let mut x = match (self.flags & 0x300) >> 8 {
            1 => {
                -if let Some(count) = center {
                    self.width(text, Some(count), letters)
                } else {
                    self.width(text, None, letters) / 2.
                }
            }
            2 => -self.width(text, None, letters),
            _ => 0.,
        };
        for &byte in text.iter().take_while(|&&b| b != 0) {
            let letter = letters[byte as usize];
            let height = letter.uv[3] - letter.uv[1];
            if geometry.vertices.len() < 4096 * 4 && height > 0. {
                let x0 = x + letter.pre_space * self.size[0] / height;
                let width = (letter.uv[2] - letter.uv[0]) / height;
                let (y0, y1, z0, z1) = if self.radius == 0. {
                    (y, y - self.size[1], 0., 0.)
                } else {
                    let f = 1. / self.radius;
                    let a = f64::from(f * y);
                    let b = f64::from(f * (y - self.size[1]));
                    let radius = f64::from(self.radius);
                    (
                        (a.sin() * radius) as f32,
                        (b.sin() * radius) as f32,
                        (-a.cos() * radius) as f32,
                        (-b.cos() * radius) as f32,
                    )
                };
                let x1 = x0 + width * self.size[0];
                let [u0, v0, u1, v1] = letter.uv;
                let base = geometry.vertices.len() as u16;
                for (x, y, z, u, v) in [
                    (x0, y0, z0, u0, v0),
                    (x1, y0, z0, u1, v0),
                    (x0, y1, z1, u0, v1),
                    (x1, y1, z1, u1, v1),
                ] {
                    geometry
                        .vertices
                        .push([x, y, z, 0., 0., f32::from_bits(u32::MAX), u, v]);
                }
                geometry
                    .indices
                    .extend([base, base + 1, base + 3, base, base + 3, base + 2]);
            }
            x += self.character_width(byte, letters) + self.space[0];
        }
    }
}
