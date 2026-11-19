//! Original EffectPrintInfo glyph metrics and TSPACE3 geometry.
use crate::bitmap::Letter;

pub struct Context<'a> {
    pub camera: &'a super::camera::Environment,
    pub pages: &'a std::collections::BTreeMap<u8, crate::bitmap::FontPage>,
    pub material_passes: Option<usize>,
    pub fps: f32,
}
pub struct Draw {
    pub page: u8,
    pub environment: super::camera::Environment,
    pub geometry: Geometry,
}
pub fn draw(
    words: &[u32],
    text: Option<&[u8]>,
    matrix: crate::mesh::Matrix,
    context: &Context<'_>,
    values: &Values<'_>,
) -> Result<Option<Draw>, &'static str> {
    draw_for_layout(
        words,
        text,
        matrix,
        context,
        values,
        crate::kx::Layout::Current,
    )
}

pub fn draw_for_layout(
    words: &[u32],
    text: Option<&[u8]>,
    matrix: crate::mesh::Matrix,
    context: &Context<'_>,
    values: &Values<'_>,
    document_layout: crate::kx::Layout,
) -> Result<Option<Draw>, &'static str> {
    let layout = Layout::from_words_for_layout(words, document_layout)?;
    if context.material_passes.unwrap_or(0) == 0 {
        return Ok(None);
    }
    let page = ((layout.flags >> 4) & 7) as u8;
    let letters = &context
        .pages
        .get(&page)
        .ok_or("Print font atlas has not been prepared")?
        .letters;
    let text = text.ok_or("missing Print string")?;
    let geometry = layout.generate(text, values, letters)?;
    Ok(Some(Draw {
        page,
        environment: layout.environment(context.camera, matrix),
        geometry,
    }))
}

pub struct Values<'a> {
    pub inventory: &'a crate::game::Inventory,
    pub current_weapon: usize,
    pub switches: &'a [u8; 256],
    pub fps: f32,
}
#[derive(Debug)]
pub struct Line {
    pub text: Vec<u8>,
    pub center: Option<usize>,
    pub newlines: usize,
}

/// Expand the original byte-oriented Print language. Reject overflow of its
/// 512-byte line buffer rather than reproduce the source's memory overwrite.
pub fn format_lines(text: &[u8], values: &Values<'_>) -> Result<Vec<Line>, &'static str> {
    fn byte(text: &[u8], p: usize) -> u8 {
        text.get(p).copied().unwrap_or(0)
    }
    fn number(text: &[u8], p: &mut usize) -> i32 {
        let mut value = 0i32;
        while byte(text, *p).is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add(i32::from(byte(text, *p) - b'0'));
            *p += 1;
        }
        value
    }
    let mut p = 0;
    let mut lines = Vec::new();
    while byte(text, p) != 0 {
        let mut line = Line {
            text: Vec::new(),
            center: None,
            newlines: 0,
        };
        while byte(text, p) != 0 && byte(text, p) != b'\n' {
            if byte(text, p) == b'%'
                && byte(text, p + 1).is_ascii_digit()
                && byte(text, p + 2) != 0
                && byte(text, p + 3) != 0
            {
                let len = usize::from(byte(text, p + 1) - b'0');
                let what = byte(text, p + 2);
                p += 3;
                let index = number(text, &mut p);
                if byte(text, p) == b';' {
                    p += 1;
                }
                let mut value = match what {
                    b'a' => values.inventory.amounts[1],
                    b'l' => values.inventory.amounts[0],
                    b's' => i32::from(values.switches[(index & 255) as usize]),
                    b'w' => values.inventory.amounts[4 + (index & 3) as usize],
                    b'W' => values.inventory.amounts[4 + ((values.current_weapon / 2) & 3)],
                    b'f' => {
                        let scaled = values.fps * 100.;
                        if (-2147483648.0..2147483648.0).contains(&scaled) {
                            scaled as i32
                        } else {
                            i32::MIN
                        }
                    }
                    _ => 0,
                };
                if what == b'W' && values.current_weapon == 0 {
                    line.text.extend_from_slice(b"inf");
                } else {
                    let start = line.text.len();
                    line.text.resize(start + len, 0);
                    for i in 0..len {
                        line.text[start + len - i - 1] = (value % 10 + 48) as u8;
                        value /= 10;
                    }
                }
            } else if byte(text, p) == b'%' && matches!(byte(text, p + 1), b'i' | b's') {
                let mode = byte(text, p + 1);
                p += 2;
                let index = number(text, &mut p);
                let mut value = i32::from(values.switches[(index & 255) as usize]);
                if mode == b'i' {
                    let mut cond = false;
                    if byte(text, p) == b'!' {
                        cond = true;
                        p += 1;
                    }
                    if byte(text, p) == b'=' {
                        p += 1;
                    } else {
                        cond = true;
                    }
                    let compare = number(text, &mut p);
                    value = i32::from(value == compare);
                    if !cond {
                        value = i32::from(value == 0);
                    }
                }
                let mut current = 0i32;
                while byte(text, p) != 0 && byte(text, p) != b';' {
                    if byte(text, p) == b'|' {
                        current = current.wrapping_add(1);
                    } else {
                        if byte(text, p) == b'\\' && byte(text, p + 1) != 0 {
                            p += 1;
                        }
                        if current == value {
                            line.text.push(byte(text, p));
                        }
                    }
                    p += 1;
                }
                if byte(text, p) == b';' {
                    p += 1;
                }
            } else if byte(text, p) == b'%' && byte(text, p + 1) == b'|' {
                line.center = Some(line.text.len());
                p += 2;
            } else {
                line.text.push(byte(text, p));
                p += 1;
            }
            if line.text.len() >= 512 {
                return Err("print line exceeds original buffer");
            }
        }
        while byte(text, p) == b'\n' {
            line.newlines += 1;
            p += 1;
        }
        lines.push(line);
    }
    Ok(lines)
}

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
    pub fn from_words(words: &[u32]) -> Result<Self, &'static str> {
        Self::from_words_for_layout(words, crate::kx::Layout::Current)
    }
    pub fn from_words_for_layout(
        words: &[u32],
        layout: crate::kx::Layout,
    ) -> Result<Self, &'static str> {
        let released = layout == crate::kx::Layout::Beta2004;
        let p = words
            .get(..if released { 5 } else { 6 })
            .ok_or("truncated Print parameters")?;
        Ok(Self {
            flags: p[0],
            size: [f32::from_bits(p[1]), f32::from_bits(p[2])],
            space: [f32::from_bits(p[3]), f32::from_bits(p[4])],
            radius: if released { 0. } else { f32::from_bits(p[5]) },
        })
    }
    pub fn environment(
        &self,
        camera: &super::camera::Environment,
        model: crate::mesh::Matrix,
    ) -> super::camera::Environment {
        if self.flags & 4 != 0 {
            let mut environment = super::camera::Environment {
                orthogonal: 1,
                ..super::camera::Environment::default()
            };
            environment.model[3][0] = -1.;
            environment.model[3][1] = 1.;
            environment
        } else {
            super::camera::Environment {
                model,
                ..camera.clone()
            }
        }
    }
    /// Run the source formatter and geometry phases with the selected atlas's
    /// metrics. Newlines advance individually to preserve float rounding.
    pub fn generate(
        &self,
        text: &[u8],
        values: &Values<'_>,
        letters: &[Letter; 256],
    ) -> Result<Geometry, &'static str> {
        let mut geometry = Geometry::default();
        let mut y = 0.;
        if self.flags & 2 != 0 {
            y += self.size[1] / 2.;
        }
        for line in format_lines(text, values)? {
            self.append_line(&line.text, line.center, y, letters, &mut geometry);
            for _ in 0..line.newlines {
                y -= self.size[1] + self.space[1];
            }
        }
        Ok(geometry)
    }
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
