//! Portable outline rasterization; GDI coverage is a separate Windows path.
use crate::bitmap::{Error, Result};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};
pub(super) struct Raster<'a> {
    face: Face<'a>,
    sx: f32,
    sy: f32,
    ascent: i32,
    pub line_height: i32,
    size: [usize; 2],
    canvas: Vec<u8>,
}
impl<'a> Raster<'a> {
    pub fn new(data: &'a [u8], size: [usize; 2], vertical: i32, horizontal: i32) -> Result<Self> {
        let face = Face::parse(data, 0).map_err(|_| Error("invalid original font face"))?;

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
        // GDI GetTextMetrics uses the unsigned Windows ascent/descent,
        // not the signed sTypoAscender/sTypoDescender fields at 68/70.
        // https://learn.microsoft.com/en-us/typography/opentype/spec/os2
        let ascent = (font_word(74)
            .map(|v| v as f32)
            .unwrap_or(face.ascender() as f32)
            * sy)
            .round_ties_even() as i32;
        let descent = (font_word(76)
            .map(|v| v as f32)
            .unwrap_or(-face.descender() as f32)
            * sy)
            .round_ties_even() as i32;
        let line_height = ascent + descent;

        Ok(Self {
            face,
            sx,
            sy,
            ascent,
            line_height,
            size,
            canvas: vec![0; size[0] * size[1]],
        })
    }
    pub fn metrics(&self, byte: u8) -> Result<[i32; 3]> {
        Ok(metrics(
            &self.face,
            self.face.glyph_index(ansi(byte)).unwrap_or(GlyphId(0)),
            self.sx,
        ))
    }
    pub fn width(&self, text: &[u8]) -> Result<i32> {
        text.iter().try_fold(0, |sum, &b| {
            Ok(sum + self.metrics(b)?.into_iter().sum::<i32>())
        })
    }
    pub fn print(&mut self, position: [i32; 2], text: &[u8]) -> Result<()> {
        let mut x = position[0];
        for &byte in text {
            let glyph = self.face.glyph_index(ansi(byte)).unwrap_or(GlyphId(0));
            draw(
                &self.face,
                glyph,
                self.sx,
                self.sy,
                x as f32,
                (position[1] + self.ascent) as f32,
                &mut self.canvas,
                self.size[0],
                self.size[1],
            );
            x += metrics(&self.face, glyph, self.sx).into_iter().sum::<i32>();
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>> {
        Ok(self.canvas)
    }
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
