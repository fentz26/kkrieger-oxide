//! Released bitmap-text layout, checked against the original x86 initializer.
use super::{Letter, RasterApi, Text};
use crate::{
    bitmap::{Error, Result},
    rounding::{add_down, div_down, mul_down},
};
#[derive(Clone, Copy)]
pub(super) struct Setup {
    pub size: [usize; 2],
    pub font_size: [i32; 2],
    pub origin: [i32; 2],
    pub padding: i32,
    pub spacing: i32,
}
impl Setup {
    pub fn new(size: [usize; 2], p: Text<'_>) -> Self {
        let w = size[0] as f32;
        let h = size[1] as f32;
        let padding = p.external_space.wrapping_mul(4);
        Self {
            size,
            font_size: [
                mul_down(h, p.width).floor() as i32,
                mul_down(w, p.height).floor() as i32,
            ],
            origin: [
                add_down(mul_down(w, p.position[0]), padding as f32).floor() as i32,
                add_down(mul_down(h, p.position[1]), padding as f32).floor() as i32,
            ],
            padding,
            spacing: mul_down(w, p.internal_space).floor() as i32,
        }
    }
}
pub(super) fn paint(
    p: Text<'_>,
    s: Setup,
    height: i32,
    raster: &mut impl RasterApi,
) -> Result<[Letter; 256]> {
    let text = &p.text[..p.text.iter().position(|&b| b == 0).unwrap_or(p.text.len())];
    let mut letters = [Letter::default(); 256];
    let [mut x, mut y] = s.origin;
    if p.flags & 0x70 == 0 {
        if p.flags & 1 != 0 {
            x -= raster.width(text)? / 2;
        }
        raster.print([x, y], text)?;
        return Ok(letters);
    }
    let mut at = 0;
    let mut limit = 0u8;
    let mut ch = b'a';
    while at < text.len() {
        let next = text.get(at + 1).copied().unwrap_or(0);
        if text[at] == b'-' && next != b'-' && next != 0 {
            limit = next;
            at += 2;
        }
        if limit != 0 {
            ch = ch.wrapping_add(1);
            if ch as i8 >= limit as i8 {
                limit = 0;
            }
        } else {
            ch = *text.get(at).unwrap_or(&0);
            at += 1;
        }
        // The released table uses a signed char index into adjacent pages.
        // Authored beta strings are ASCII; do not emulate an out-of-page write.
        if ch >= 128 {
            return Err(Error("released font byte indexes an adjacent page"));
        }
        let advance = raster.width(&[ch])? + s.spacing * 2;
        if x + advance + s.padding >= s.size[0] as i32 {
            y = ((y + height + s.spacing * 2 + 3) & !3) + s.padding;
            x = s.origin[0];
        }
        letters[usize::from(ch)].uv = [
            div_down(x as f32, s.size[0] as f32),
            div_down(y as f32, s.size[1] as f32),
            div_down((x + advance) as f32, s.size[0] as f32),
            div_down((y + height + s.spacing * 2) as f32, s.size[1] as f32),
        ];
        raster.print([x + s.spacing, y + s.spacing], &[ch])?;
        x = ((x + advance + 3) & !3) + s.padding;
    }
    Ok(letters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::Text;
    struct Reader<'a>(&'a [u8]);
    impl Reader<'_> {
        fn word(&mut self) -> u32 {
            let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            n
        }
        fn bytes(&mut self, n: usize) -> Vec<u8> {
            let b = self.0[..n].to_vec();
            self.0 = &self.0[n..];
            b
        }
    }
    struct Fake {
        calls: Vec<([i32; 2], Vec<u8>)>,
    }
    impl super::super::RasterApi for Fake {
        fn width(&self, text: &[u8]) -> crate::bitmap::Result<i32> {
            Ok(text.iter().map(|b| i32::from(b % 13 + 1)).sum())
        }
        fn print(&mut self, position: [i32; 2], text: &[u8]) -> crate::bitmap::Result<()> {
            self.calls.push((position, text.to_vec()));
            Ok(())
        }
    }
    #[test]
    fn released_layout_calls_and_uv_tables_match_native_initializer() {
        let mut r = Reader(include_bytes!(
            "../../../tests/fixtures/beta-font-layout.bin"
        ));
        assert_eq!(r.word(), 96);
        for case in 0..96 {
            let size = [r.word() as usize * 4, r.word() as usize * 4];
            let height = r.word() as i32;
            let words: [u32; 8] = std::array::from_fn(|_| r.word());
            let n = r.word() as usize;
            let text = r.bytes(n);
            let p = Text {
                position: [f32::from_bits(words[0]), f32::from_bits(words[1])],
                width: f32::from_bits(words[2]),
                height: f32::from_bits(words[3]),
                color: words[4],
                flags: words[5],
                external_space: words[6] as i32,
                internal_space: f32::from_bits(words[7]),
                line_skip: 1.,
                text: &text,
                font: b"Arial",
                blob: &[],
            };
            let setup = Setup::new(size, p);
            for value in [
                size[0] as i32,
                size[1] as i32,
                setup.font_size[1],
                setup.font_size[0],
                4,
            ] {
                assert_eq!(value as u32, r.word(), "case{case} FontBegin");
            }
            let n = r.word() as usize;
            let expected: Vec<_> = (0..n)
                .map(|_| {
                    let pos = [r.word() as i32, r.word() as i32];
                    let n = r.word() as usize;
                    (pos, r.bytes(n))
                })
                .collect();
            let mut fake = Fake { calls: Vec::new() };
            let letters = paint(p, setup, height, &mut fake).unwrap();
            assert_eq!(fake.calls, expected, "case{case} draws");
            for (glyph, letter) in letters.iter().enumerate() {
                for value in letter.uv {
                    assert_eq!(value.to_bits(), r.word(), "case{case}/{glyph}");
                }
            }
        }
        assert!(r.0.is_empty());
    }
}
