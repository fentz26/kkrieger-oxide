//! Original text/atlas layout with a Rust outline rasterizer.
//! Layout and downsampling derive from Bitmap_Text; font outlines come from
//! the same named system font. GDI hinting/raster coverage parity remains a
//! separate reference check, rather than silently substituting other fonts.
use super::*;
use std::{collections::HashMap, path::PathBuf};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

#[derive(Clone, Copy, Debug, Default)]
pub struct Letter {
    pub uv: [f32; 4],
    pub pre_space: f32,
    pub width: f32,
    pub origin: [u16; 2],
    pub abc: [i16; 3],
}
#[derive(Clone, Debug)]
pub struct FontPage {
    pub page: u8,
    pub line_height: u16,
    pub letters: [Letter; 256],
}
#[derive(Clone, Copy)]
pub struct Text<'a> {
    pub position: [f32; 2],
    pub width: f32,
    pub height: f32,
    pub color: u32,
    pub flags: u32,
    pub external_space: i32,
    pub internal_space: f32,
    pub line_skip: f32,
    pub text: &'a [u8],
    pub font: &'a [u8],
    pub blob: &'a [u8],
}
#[derive(Default)]
pub struct FontBook {
    fonts: HashMap<String, Vec<u8>>,
}
impl FontBook {
    pub fn new() -> Self {
        Self::default()
    }
    fn get(&mut self, name: &[u8]) -> Result<&[u8]> {
        let name = String::from_utf8_lossy(name).to_ascii_lowercase();
        let name = if name.is_empty() {
            "arial".to_owned()
        } else {
            name
        };
        if !self.fonts.contains_key(&name) {
            let filenames: &[&str] = match name.as_str() {
                "arial" => &["Arial.ttf", "arial.ttf"],
                "times new roman" => &["Times New Roman.ttf", "times.ttf", "Times_New_Roman.ttf"],
                _ => return Err(Error("original requested font is not installed")),
            };
            let mut dirs = Vec::new();
            if let Some(dir) = std::env::var_os("KKRIEGER_FONT_DIR") {
                dirs.push(PathBuf::from(dir));
            }
            dirs.extend([
                PathBuf::from("/System/Library/Fonts/Supplemental"),
                PathBuf::from("C:/Windows/Fonts"),
                PathBuf::from("/usr/share/fonts/truetype/msttcorefonts"),
            ]);
            let mut data = None;
            for dir in dirs {
                for file in filenames {
                    if let Ok(bytes) = std::fs::read(dir.join(file)) {
                        data = Some(bytes);
                        break;
                    }
                }
                if data.is_some() {
                    break;
                }
            }
            self.fonts.insert(
                name.clone(),
                data.ok_or(Error("original requested font is not installed"))?,
            );
        }
        Ok(&self.fonts[&name])
    }
}

impl Bitmap {
    pub fn text(&mut self, p: Text<'_>, fonts: &mut FontBook) -> Result<()> {
        let page = ((p.flags & 0x70) >> 4) as u8;
        let width = self
            .width
            .checked_mul(4)
            .ok_or(Error("text canvas overflow"))?;
        let height = self
            .height
            .checked_mul(4)
            .ok_or(Error("text canvas overflow"))?;
        if width > 16384 || height > 16384 {
            return Err(Error("text canvas exceeds bitmap limit"));
        }
        let spacing = (p.internal_space * width as f32) as i32;
        let mut letters = [Letter::default(); 256];
        let line_height;
        let mut mask;
        let header_valid = p.blob.len() >= 8
            && p.blob[0] == 3
            && p.blob[1] == u8::from(page != 0)
            && u16::from_le_bytes([p.blob[2], p.blob[3]]) as usize == self.width
            && u16::from_le_bytes([p.blob[4], p.blob[5]]) as usize == self.height;
        if p.flags & 0x80 != 0 && header_valid {
            line_height = u16::from_le_bytes([p.blob[6], p.blob[7]]) as i32;
            let mut offset = 8;
            if page != 0 {
                if p.blob.len() < offset + 2560 {
                    return Err(Error("truncated original font metrics"));
                }
                for letter in &mut letters {
                    let word = |i| u16::from_le_bytes([p.blob[offset + i], p.blob[offset + i + 1]]);
                    letter.origin = [word(0), word(2)];
                    letter.abc = [word(4) as i16, word(6) as i16, word(8) as i16];
                    offset += 10;
                }
            }
            let data = p
                .blob
                .get(offset..offset + self.pixels.len())
                .ok_or(Error("truncated original font coverage"))?;
            if data.iter().any(|&v| v > 7) {
                return Err(Error("invalid compressed original font coverage"));
            }
            mask = data
                .iter()
                .map(|&v| (v << 5) | (v << 2) | (v >> 1))
                .collect::<Vec<_>>();
        } else {
            let data = fonts.get(p.font)?;
            let face = Face::parse(data, 0).map_err(|_| Error("invalid original font face"))?;
            let vertical = (height as f32 * p.width) as i32;
            let horizontal = (width as f32 * p.height) as i32;
            if vertical <= 0 {
                return Err(Error("zero original font height"));
            }
            let sy = vertical as f32 / face.units_per_em() as f32;
            let os2 = face.raw_face().table(Tag::from_bytes(b"OS/2"));
            let font_word = |offset| {
                os2.and_then(|b| b.get(offset..offset + 2))
                    .map(|s| u16::from_be_bytes(s.try_into().unwrap()))
            };
            let average = font_word(2)
                .map(|v| v as i16 as f32)
                .filter(|&v| v > 0.)
                .unwrap_or(face.units_per_em() as f32 * 0.5);
            let sx = if horizontal == 0 {
                sy
            } else {
                horizontal as f32 / average
            };
            let ascent = (font_word(68)
                .map(|v| v as f32)
                .unwrap_or(face.ascender() as f32)
                * sy)
                .round_ties_even() as i32;
            let descent = (font_word(70)
                .map(|v| v as f32)
                .unwrap_or(-face.descender() as f32)
                * sy)
                .round_ties_even() as i32;
            line_height = ascent + descent;
            let es = p.external_space * 4 + 8;
            let origin = (p.position[0] * width as f32) as i32 + es;
            let mut x = origin;
            let mut y = (p.position[1] * height as f32) as i32 + es;
            let mut canvas = vec![0_u8; width * height];
            if page != 0 {
                for byte in expanded_range(p.text) {
                    let glyph = face.glyph_index(ansi(byte)).unwrap_or(GlyphId(0));
                    let abc = metrics(&face, glyph, sx);
                    if x + abc[1] + es + 2 * spacing >= width as i32 {
                        y = (y + line_height + 2 * spacing + 3) & !3;
                        y += es;
                        x = origin;
                        if y + line_height + es + 2 * spacing >= height as i32 {
                            break;
                        }
                    }
                    letters[byte as usize].origin = [x as u16, y as u16];
                    letters[byte as usize].abc = abc.map(|v| v as i16);
                    draw(
                        &face,
                        glyph,
                        sx,
                        sy,
                        (x + spacing - abc[0]) as f32,
                        (y + spacing + ascent) as f32,
                        &mut canvas,
                        width,
                        height,
                    );
                    x = (x + abc[1] + 2 * spacing + 3) & !3;
                    x += es;
                }
            } else {
                if p.flags & 2 != 0 {
                    y -= line_height * (p.text.iter().filter(|&&b| b == b'\n').count() as i32 + 1)
                        / 2;
                }
                for line in p.text.split(|&b| b == b'\n') {
                    let advances: Vec<_> = line
                        .iter()
                        .map(|&b| {
                            let id = face.glyph_index(ansi(b)).unwrap_or(GlyphId(0));
                            (id, metrics(&face, id, sx)[0..3].iter().sum::<i32>())
                        })
                        .collect();
                    let mut cursor = if p.flags & 1 != 0 {
                        x - advances.iter().map(|v| v.1).sum::<i32>() / 2
                    } else {
                        x
                    };
                    for (glyph, advance) in advances {
                        draw(
                            &face,
                            glyph,
                            sx,
                            sy,
                            cursor as f32,
                            (y + ascent) as f32,
                            &mut canvas,
                            width,
                            height,
                        );
                        cursor += advance;
                    }
                    y += (p.line_skip * line_height as f32) as i32;
                }
            }
            mask = Vec::with_capacity(self.pixels.len());
            for y in 0..self.height {
                for x in 0..self.width {
                    let mut sum = 0_u32;
                    for yy in 0..4 {
                        for xx in 0..4 {
                            sum += canvas[(y * 4 + yy) * width + x * 4 + xx] as u32;
                        }
                    }
                    mask.push((sum >> 4) as u8);
                }
            }
        }
        let color = color64(p.color);
        for (pixel, coverage) in self.pixels.iter_mut().zip(mask.drain(..)) {
            *pixel = fade64(*pixel, color, coverage as i32 * 257);
        }
        if page != 0 {
            for letter in &mut letters {
                let [x, y] = letter.origin;
                let [a, b, c] = letter.abc;
                letter.uv = [
                    x as f32 / width as f32,
                    y as f32 / height as f32,
                    (x as i32 + b as i32 + 2 * spacing) as f32 / width as f32,
                    (y as i32 + line_height + 2 * spacing) as f32 / height as f32,
                ];
                letter.pre_space = (a as i32 - spacing) as f32 / width as f32;
                letter.width = (a as i32 + b as i32 + c as i32 + 2 * spacing) as f32 / width as f32;
            }
            self.font_page = Some(Box::new(FontPage {
                page,
                line_height: line_height as u16,
                letters,
            }));
        }
        Ok(())
    }
}

fn expanded_range(text: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut p = 0;
    let mut limit = 0_u8;
    let mut ch = b'a';
    while p < text.len() || ch < limit {
        if text.get(p) == Some(&b'-') && text.get(p + 1).is_some_and(|&c| c != b'-') {
            limit = text[p + 1];
            p += 2;
        }
        if text.get(p) == Some(&b'-') && text.get(p + 1) == Some(&b'-') {
            p += 1;
        }
        if ch < limit {
            ch += 1;
        } else {
            limit = 0;
            if p == text.len() {
                break;
            }
            ch = text[p];
            p += 1;
        }
        result.push(ch);
    }
    result
}
fn ansi(c: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    if (128..160).contains(&c) {
        HIGH[c as usize - 128]
    } else {
        c as char
    }
}
fn metrics(face: &Face<'_>, id: GlyphId, sx: f32) -> [i32; 3] {
    let advance = (face.glyph_hor_advance(id).unwrap_or(0) as f32 * sx).round_ties_even() as i32;
    let bounds = face.glyph_bounding_box(id);
    let a = bounds
        .map(|b| (b.x_min as f32 * sx).floor() as i32)
        .unwrap_or(0);
    let b = bounds
        .map(|b| (b.x_max as f32 * sx).ceil() as i32 - a)
        .unwrap_or(0);
    [a, b, advance - a - b]
}

type Point = [f32; 2];
struct Outline {
    edges: Vec<(Point, Point)>,
    at: Point,
    start: Point,
    scale: Point,
    origin: Point,
}
impl Outline {
    fn point(&self, x: f32, y: f32) -> Point {
        [
            self.origin[0] + x * self.scale[0],
            self.origin[1] - y * self.scale[1],
        ]
    }
    fn line(&mut self, p: Point) {
        if self.at != p {
            self.edges.push((self.at, p));
        }
        self.at = p;
    }
    fn quad(&mut self, a: Point, b: Point, c: Point, depth: u32) {
        let d = [a[0] - 2. * b[0] + c[0], a[1] - 2. * b[1] + c[1]];
        if depth == 12 || d[0] * d[0] + d[1] * d[1] < 0.04 {
            self.line(c);
            return;
        }
        let ab = mid(a, b);
        let bc = mid(b, c);
        let m = mid(ab, bc);
        self.quad(a, ab, m, depth + 1);
        self.quad(m, bc, c, depth + 1);
    }
    fn cubic(&mut self, a: Point, b: Point, c: Point, d: Point, depth: u32) {
        let u = [3. * b[0] - 2. * a[0] - d[0], 3. * b[1] - 2. * a[1] - d[1]];
        let v = [3. * c[0] - 2. * d[0] - a[0], 3. * c[1] - 2. * d[1] - a[1]];
        if depth == 12 || u[0].abs().max(v[0].abs()) + u[1].abs().max(v[1].abs()) < 0.2 {
            self.line(d);
            return;
        }
        let ab = mid(a, b);
        let bc = mid(b, c);
        let cd = mid(c, d);
        let abc = mid(ab, bc);
        let bcd = mid(bc, cd);
        let m = mid(abc, bcd);
        self.cubic(a, ab, abc, m, depth + 1);
        self.cubic(m, bcd, cd, d, depth + 1);
    }
}
impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.at = self.point(x, y);
        self.start = self.at;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.line(self.point(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.quad(self.at, self.point(x1, y1), self.point(x, y), 0);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.cubic(
            self.at,
            self.point(x1, y1),
            self.point(x2, y2),
            self.point(x, y),
            0,
        );
    }
    fn close(&mut self) {
        self.line(self.start);
    }
}
fn mid(a: Point, b: Point) -> Point {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}
#[allow(clippy::too_many_arguments)]
fn draw(
    face: &Face<'_>,
    glyph: GlyphId,
    sx: f32,
    sy: f32,
    x: f32,
    baseline: f32,
    canvas: &mut [u8],
    width: usize,
    height: usize,
) {
    let mut outline = Outline {
        edges: Vec::new(),
        at: [0.; 2],
        start: [0.; 2],
        scale: [sx, sy],
        origin: [x, baseline],
    };
    if face.outline_glyph(glyph, &mut outline).is_none() {
        return;
    }
    let low = outline
        .edges
        .iter()
        .map(|(a, b)| a[1].min(b[1]))
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.) as usize;
    let high = (outline
        .edges
        .iter()
        .map(|(a, b)| a[1].max(b[1]))
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil() as usize)
        .min(height);
    let mut intersections = Vec::with_capacity(outline.edges.len());
    let mut row = vec![0_f32; width];
    for y in low..high {
        row.fill(0.);
        for sample in 0..4 {
            let scan = y as f32 + (sample as f32 + 0.5) * 0.25;
            intersections.clear();
            for &(a, b) in &outline.edges {
                if (a[1] <= scan && b[1] > scan) || (b[1] <= scan && a[1] > scan) {
                    intersections.push((
                        a[0] + (scan - a[1]) * (b[0] - a[0]) / (b[1] - a[1]),
                        if b[1] > a[1] { 1 } else { -1 },
                    ));
                }
            }
            intersections.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            let mut start = 0.;
            for &(x, direction) in &intersections {
                if winding == 0 {
                    start = x;
                }
                winding += direction;
                if winding == 0 {
                    let left = start.max(0.);
                    let right = x.min(width as f32);
                    if right > left {
                        for (i, pixel) in row
                            .iter_mut()
                            .enumerate()
                            .take(right.ceil() as usize)
                            .skip(left.floor() as usize)
                        {
                            *pixel +=
                                (right.min(i as f32 + 1.) - left.max(i as f32)).max(0.) * 0.25;
                        }
                    }
                }
            }
        }
        for (x, coverage) in row.iter().enumerate() {
            let a = (coverage * 255.).round().clamp(0., 255.) as u32;
            let pixel = &mut canvas[y * width + x];
            *pixel = (*pixel as u32 + a - (*pixel as u32 * a) / 255).min(255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_character_range_language_preserves_ranges_and_hyphens() {
        assert_eq!(expanded_range(b"a-cA-C0-2!"), b"abcABC012!");
        assert_eq!(expanded_range(b"--!"), b"-!");
        assert_eq!(
            expanded_range(b"a-zA-Z0-9.,-:;><! "),
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,-./0123456789:;><! "
        );
    }
    #[test]
    fn original_cached_font_coverage_uses_exact_integer_blending() {
        let mut b = Bitmap::flat(0, 0, 0xff000000, 0);
        let mut blob = vec![3, 0, 1, 0, 1, 0, 20, 0];
        blob.push(7);
        b.text(
            Text {
                position: [0.; 2],
                width: 0.,
                height: 0.,
                color: 0xffffffff,
                flags: 128,
                external_space: 0,
                internal_space: 0.,
                line_skip: 1.,
                text: b"",
                font: b"",
                blob: &blob,
            },
            &mut FontBook::new(),
        )
        .unwrap();
        assert_eq!(
            b.pixels[0],
            fade64(color64(0xff000000), color64(0xffffffff), 65535)
        );
    }
}
