//! Original 15-bit BGRA texture arithmetic, translated from MMX to portable Rust.
//! Farbrausch BSD-2-Clause: third_party/farbrausch-LICENSE.txt.
use crate::random::{Noise, Random};
use std::fmt;
mod filters;
mod text;
pub use filters::{Bump, Cell, GlowRect, Transform, Twirl};
pub use text::{FontBook, FontPage, Letter, Text};
pub type Pixel = [u16; 4];
const WHITE: u16 = 0x7fff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Difference,
    Alpha,
    MultiplyColor,
    AddColor,
    SubtractColor,
    Gray,
    Invert,
    ScaleColor,
    Merge,
    Brightness,
    ReverseSubtract,
    MultiplyMerge,
    Sharpen,
    HardLight,
    Over,
    Screen,
    Range,
}
impl TryFrom<u8> for Operation {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        use Operation::*;
        Ok(match value {
            0 => Add,
            1 => Subtract,
            2 => Multiply,
            3 => Difference,
            4 => Alpha,
            5 => MultiplyColor,
            6 => AddColor,
            7 => SubtractColor,
            8 => Gray,
            9 => Invert,
            10 => ScaleColor,
            11 => Merge,
            12 => Brightness,
            13 => ReverseSubtract,
            14 => MultiplyMerge,
            15 => Sharpen,
            16 => HardLight,
            17 => Over,
            18 => Screen,
            19 => Range,
            _ => return Err(Error("unknown pixel operation")),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Bitmap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Pixel>,
    pub format: u32,
    pub mip_count: u32,
    pub mip_threshold: u32,
    pub font_page: Option<Box<FontPage>>,
}
impl Bitmap {
    pub fn flat(x_exp: i32, y_exp: i32, color: u32, offset: i32) -> Self {
        let width = 1_usize << x_exp.saturating_add(offset).clamp(0, 12);
        let height = 1_usize << y_exp.saturating_add(offset).clamp(0, 12);
        Self {
            width,
            height,
            pixels: vec![color64(color); width * height],
            format: 1,
            mip_count: 0,
            mip_threshold: 0,
            font_page: None,
        }
    }
    pub fn color(&mut self, mode: u32, color: u32) -> Result<()> {
        if mode > 5 {
            return Err(Error("invalid color mode"));
        }
        let op = Operation::try_from(mode as u8 + 5)?;
        let color = color64(color);
        for pixel in &mut self.pixels {
            *pixel = process(op, *pixel, color, *pixel, color);
        }
        Ok(())
    }
    pub fn merge(&mut self, other: &Self, mode: u32) -> Result<()> {
        self.check_size(other)?;
        let op = match mode {
            0..=4 => Operation::try_from(mode as u8)?,
            5 => Operation::Brightness,
            6 => Operation::HardLight,
            7 => Operation::Over,
            8 => Operation::Screen,
            _ => return Err(Error("invalid merge mode")),
        };
        for (pixel, &src) in self.pixels.iter_mut().zip(&other.pixels) {
            *pixel = process(op, *pixel, src, *pixel, src);
        }
        Ok(())
    }
    pub fn range(&mut self, mode: u32, low: u32, high: u32) -> Result<()> {
        if mode > 5 {
            return Err(Error("invalid range mode"));
        }
        let a = color64(low);
        let b = color64(high);
        for p in &mut self.pixels {
            if mode & 1 != 0 {
                *p = process(Operation::Gray, *p, a, *p, b);
            }
            if mode & 2 != 0 {
                *p = process(Operation::Invert, *p, a, *p, b);
            }
            *p = process(Operation::Range, *p, a, *p, b);
        }
        Ok(())
    }
    pub fn mask(&mut self, a: &Self, b: &Self, mode: u32) -> Result<()> {
        self.check_size(a)?;
        self.check_size(b)?;
        if mode > 3 {
            return Err(Error("invalid mask mode"));
        }
        for ((p, &a), &b) in self.pixels.iter_mut().zip(&a.pixels).zip(&b.pixels) {
            let mask = process(Operation::Gray, *p, *p, *p, *p);
            *p = match mode {
                0 => process(Operation::Merge, mask, a, b, a),
                1 => {
                    let m = process(Operation::Multiply, mask, a, mask, a);
                    process(Operation::Add, m, b, m, b)
                }
                2 => {
                    let m = process(Operation::Multiply, mask, a, mask, a);
                    process(Operation::ReverseSubtract, m, b, m, b)
                }
                _ => process(Operation::MultiplyMerge, mask, a, b, a),
            };
        }
        Ok(())
    }
    fn check_size(&self, b: &Self) -> Result<()> {
        if self.width != b.width || self.height != b.height {
            Err(Error("bitmap dimensions differ"))
        } else {
            Ok(())
        }
    }
    pub fn rgba8(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for &[b, g, r, a] in &self.pixels {
            bytes.extend([
                (r >> 7).min(255) as u8,
                (g >> 7).min(255) as u8,
                (b >> 7).min(255) as u8,
                (a >> 7).min(255) as u8,
            ]);
        }
        bytes
    }

    pub fn hscb(&mut self, hue: f32, saturation: f32, contrast: f32, brightness: f32) {
        let contrast = contrast * contrast;
        let table: [i32; 1025] = std::array::from_fn(|i| {
            ((((i * 32) as f64 + 0.01) / 32768.).powf(contrast as f64) * 32768. * brightness as f64)
                as i32
        });
        let hue = (hue * 6. * 65536.) as i32 % (6 * 65536);
        let saturation = (saturation * 65536.) as i32;
        for p in &mut self.pixels {
            let (mut r, mut g, mut b) = (
                gamma(&table, p[2] as i32),
                gamma(&table, p[1] as i32),
                gamma(&table, p[0] as i32),
            );
            if hue != 0 || saturation != 65536 {
                let max = r.max(g).max(b);
                let min = r.min(g).min(b);
                let mut difference = max - min;
                let mut ch = if difference == 0 {
                    0
                } else if r == max {
                    65536 + div_shift(g - b, difference)
                } else if g == max {
                    3 * 65536 + div_shift(b - r, difference)
                } else {
                    5 * 65536 + div_shift(r - g, difference)
                };
                ch += hue;
                difference = mul_shift(difference, saturation);
                let min = max - difference;
                if ch > 6 * 65536 {
                    ch -= 6 * 65536;
                }
                let rem = ch & 131071;
                let m1 = min
                    + if rem >= 65536 {
                        mul_shift(rem - 65536, difference)
                    } else {
                        0
                    };
                let m2 = min
                    + if rem < 65536 {
                        mul_shift(65536 - rem, difference)
                    } else {
                        0
                    };
                if ch < 2 * 65536 {
                    r = max;
                    g = m1;
                    b = m2;
                } else if ch < 4 * 65536 {
                    r = m2;
                    g = max;
                    b = m1;
                } else {
                    r = m1;
                    g = m2;
                    b = max;
                }
            }
            p[2] = r.clamp(0, 32767) as u16;
            p[1] = g.clamp(0, 32767) as u16;
            p[0] = b.clamp(0, 32767) as u16;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn perlin(
        xs: i32,
        ys: i32,
        frequency: i32,
        octaves: i32,
        fadeoff: f32,
        seed: u8,
        mode: u32,
        amplitude: f32,
        gamma_power: f32,
        low: u32,
        high: u32,
        offset: i32,
        noise: &Noise,
    ) -> Result<Self> {
        if frequency < 0 || octaves < 0 || frequency.saturating_add(octaves) > 31 || mode > 3 {
            return Err(Error("invalid Perlin parameters"));
        }
        let mut bm = Self::flat(xs, ys, 0, offset);
        let a = color64(low);
        let b = color64(high);
        let sx = bm.width.ilog2();
        let sy = bm.height.ilog2();
        let table: [i32; 1025] = std::array::from_fn(|i| {
            (((i as f32 / 1024.) as f64).powf(gamma_power as f64) * 32768.) as i32
        })
        .map(|i| i.clamp(0, 32767) * 2);
        let amplitude = amplitude * if mode & 1 != 0 { 32768. } else { 16384. };
        let offset = if mode & 1 != 0 { 0 } else { 16384 };
        for y in 0..bm.height {
            for x in 0..bm.width {
                let mut n = 0.;
                let mut scale = 1.;
                for octave in frequency..frequency + octaves {
                    let px = ((x as i32) << (16 - sx)).wrapping_shl(octave as u32);
                    let py = ((y as i32) << (16 - sy)).wrapping_shl(octave as u32);
                    let mut v = noise.sample2d(
                        px,
                        py,
                        (1_i32.wrapping_shl(octave as u32)).wrapping_sub(1),
                        seed,
                    );
                    if mode & 1 != 0 {
                        v = v.abs();
                    }
                    if mode & 2 != 0 {
                        v = (((v * std::f32::consts::TAU) as f64).sin() * 0.5) as f32;
                    }
                    n += v * scale;
                    scale *= fadeoff;
                }
                let v = ((n * amplitude).round_ties_even() as i32)
                    .saturating_add(offset)
                    .clamp(0, 32767);
                bm.pixels[y * bm.width + x] = fade64(a, b, gamma(&table, v));
            }
        }
        Ok(bm)
    }

    pub fn dots(&mut self, low: u32, high: u32, density: i32, seed: i32) -> Result<()> {
        if self.pixels.is_empty() || density < 0 {
            return Err(Error("invalid dots parameters"));
        }
        let mut rng = Random::new(seed);
        let count = ((self.pixels.len() as i64 * density as i64) >> 12).max(1);
        let a = color64(low);
        let b = color64(high);
        for _ in 0..count {
            let fade = rng.below(65536) as i32;
            let i = rng.below(self.pixels.len() as u32) as usize;
            self.pixels[i] = fade64(a, b, fade);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn gradient(
        xs: i32,
        ys: i32,
        low: u32,
        high: u32,
        position: f32,
        angle: f32,
        length: f32,
        mode: u32,
        offset: i32,
    ) -> Result<Self> {
        if !length.is_finite() || length == 0. || mode > 2 {
            return Err(Error("invalid gradient parameters"));
        }
        let mut bm = Self::flat(xs, ys, 0, offset);
        let a = color64(low);
        let b = color64(high);
        let scale = 32768. / length;
        let dx = ((angle as f64 * std::f64::consts::TAU).cos() * scale as f64) as f32;
        let dy = ((angle as f64 * std::f64::consts::TAU).sin() * scale as f64) as f32;
        let dx = dx.round_ties_even() as i32;
        let dy = dy.round_ties_even() as i32;
        let cdx = mul_shift(dx, 65536 / bm.width as i32);
        let cdy = mul_shift(dy, 65536 / bm.height as i32) - bm.width as i32 * cdx;
        let mut c = (16384. - (dx / 2 + dy / 2) as f32 * (position + 1.)) as i32;
        for row in bm.pixels.chunks_mut(bm.width) {
            for p in row {
                let v = c.clamp(0, 32767);
                let fade = match mode {
                    0 => (v * 2) as f32,
                    1 => {
                        ((v as f64 * std::f64::consts::TAU / 32768. + 8192.).sin() * 32767.
                            + 32767.) as f32
                    }
                    _ => ((v as f64 * std::f64::consts::TAU / 65536.).sin() * 65535.) as f32,
                };
                *p = fade64(a, b, fade.round_ties_even() as i32);
                c = c.wrapping_add(cdx);
            }
            c = c.wrapping_add(cdy);
        }
        Ok(bm)
    }
}

fn mul_shift(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> 16) as i32
}
fn div_shift(a: i32, b: i32) -> i32 {
    (((a as i64) << 16) / b as i64) as i32
}
fn gamma(table: &[i32; 1025], value: i32) -> i32 {
    let i = (value >> 5) as usize;
    table[i] + (((table[i + 1] - table[i]) * (value & 31)) >> 5)
}

/// Exact packed-integer interpolation, including MMX's endpoint rounding.
pub fn fade64(a: Pixel, b: Pixel, fade: i32) -> Pixel {
    let f = fade >> 1;
    let wb = signed_sat(f);
    let wa = signed_sat(32768_i32.wrapping_sub(f));
    std::array::from_fn(|i| {
        mul_high(b[i], wb)
            .wrapping_add(mul_high(a[i], wa))
            .wrapping_shl(1)
    })
}

pub fn color64(argb: u32) -> Pixel {
    std::array::from_fn(|i| ((((argb >> (i * 8)) & 255) * 257) >> 1) as u16)
}
fn signed_sat(value: i32) -> u16 {
    value.clamp(-32768, 32767) as i16 as u16
}
fn add_signed(a: u16, b: u16) -> u16 {
    signed_sat(a as i16 as i32 + b as i16 as i32)
}
fn sub_signed(a: u16, b: u16) -> u16 {
    signed_sat(a as i16 as i32 - b as i16 as i32)
}
fn mul_high(a: u16, b: u16) -> u16 {
    ((a as i16 as i32 * b as i16 as i32) >> 16) as u16
}
fn multiply(a: u16, b: u16) -> u16 {
    mul_high(a, b).wrapping_shl(1)
}
fn gray(pixel: Pixel) -> u16 {
    let bg = pixel[0].wrapping_add(pixel[1]) >> 1;
    let rg = pixel[2].wrapping_add(pixel[1]) >> 1;
    bg.wrapping_add(rg) >> 1
}

/// One pixel of Bitmap_Inner. `destination` is the old destination (mask or
/// blurred input in some modes), `source` the other image, `base` the original
/// first input. Range uses `source` and `second_color` as its two endpoints.
pub fn process(
    op: Operation,
    destination: Pixel,
    source: Pixel,
    base: Pixel,
    second_color: Pixel,
) -> Pixel {
    use Operation::*;
    if matches!(op, Gray) {
        let g = gray(base);
        return [g, g, g, WHITE];
    }
    if matches!(op, Alpha) {
        return [
            base[0] & WHITE,
            base[1] & WHITE,
            base[2] & WHITE,
            gray(source),
        ];
    }
    std::array::from_fn(|i| {
        let (d, s, x) = (destination[i], source[i], base[i]);
        match op {
            Add | AddColor => add_signed(x, s),
            Subtract | SubtractColor => x.saturating_sub(s),
            Multiply | MultiplyColor => multiply(x, s),
            Difference => x.wrapping_sub(s).wrapping_add(WHITE) >> 1,
            Invert => x ^ WHITE,
            ScaleColor => signed_sat(((x as i16 as i32 * s as i16 as i32) as u32 >> 11) as i32),
            Merge => add_signed(mul_high(d, s), mul_high(sub_signed(WHITE, d), x)).wrapping_shl(1),
            Brightness => {
                let mask = if (s as i16) > 0x3fff { WHITE } else { 0 };
                mul_high(s ^ mask, x ^ mask).wrapping_shl(2) ^ mask
            }
            ReverseSubtract => s.saturating_sub(x),
            MultiplyMerge => add_signed(
                mul_high(d, multiply(s, x)),
                mul_high(sub_signed(WHITE, d), x),
            )
            .wrapping_shl(1),
            Sharpen => {
                let difference = x.saturating_sub(d);
                let scaled = signed_sat((difference as i16 as i32 * s as i16 as i32) >> 11);
                (add_signed(scaled, x) as i16).max(0) as u16
            }
            HardLight => {
                let doubled = s.wrapping_shl(1);
                let mask = ((doubled as i16) >> 15) as u16;
                let t = doubled & WHITE;
                (multiply(x, t) ^ mask).wrapping_add(t.wrapping_add(x) & mask)
            }
            Over => {
                let alpha = source[3];
                (add_signed(multiply(sub_signed(s, x), alpha), x) as i16).max(0) as u16
            }
            Screen => multiply(x ^ WHITE, s ^ WHITE) ^ WHITE,
            Range => {
                let delta = sub_signed(second_color[i], s);
                (add_signed(multiply(x, delta), s) as i16).max(0) as u16
            }
            Gray | Alpha => unreachable!(),
        }
    })
}

pub fn inner(
    destination: &mut [Pixel],
    source: &[Pixel],
    base: &[Pixel],
    op: Operation,
) -> Result<()> {
    let constant = matches!(
        op,
        Operation::MultiplyColor
            | Operation::AddColor
            | Operation::SubtractColor
            | Operation::Gray
            | Operation::Invert
            | Operation::ScaleColor
            | Operation::Sharpen
            | Operation::Range
    );
    if source.is_empty()
        || base.len() != destination.len()
        || (!constant && source.len() < destination.len())
        || (matches!(op, Operation::Range) && source.len() < 2)
    {
        return Err(Error("invalid pixel buffer lengths"));
    }
    for (i, (d, &x)) in destination.iter_mut().zip(base).enumerate() {
        let s = source[if constant { 0 } else { i }];
        *d = process(op, *d, s, x, source.get(1).copied().unwrap_or(s));
    }
    Ok(())
}
