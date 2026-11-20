//! Original text/atlas layout with native Windows GDI and portable outlines.
//! Windows uses the published system-font settings. Other platforms rasterize
//! the same named outlines; complete released pixel parity remains separate.
use super::*;
#[cfg(not(target_os = "windows"))]
use std::{collections::HashMap, path::PathBuf};
#[cfg(not(target_os = "windows"))]
mod outline;
#[cfg(target_os = "windows")]
mod windows;

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
    #[cfg(not(target_os = "windows"))]
    fonts: HashMap<String, Vec<u8>>,
}
impl FontBook {
    pub fn new() -> Self {
        Self::default()
    }
    #[cfg(not(target_os = "windows"))]
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
                data.ok_or(Error(
                    "original requested font is not installed; set KKRIEGER_FONT_DIR (see BUILDING.md)",
                ))?,
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
            let vertical = (height as f32 * p.width) as i32;
            let horizontal = (width as f32 * p.height) as i32;
            #[cfg(not(target_os = "windows"))]
            let mut raster =
                outline::Raster::new(fonts.get(p.font)?, [width, height], vertical, horizontal)?;
            #[cfg(target_os = "windows")]
            let mut raster = windows::Raster::new(p.font, [width, height], vertical, horizontal)?;
            #[cfg(target_os = "windows")]
            let _ = fonts;
            line_height = raster.line_height;
            let es = p.external_space * 4 + 8;
            let origin = (p.position[0] * width as f32) as i32 + es;
            let mut x = origin;
            let mut y = (p.position[1] * height as f32) as i32 + es;
            if page != 0 {
                for byte in expanded_range(p.text) {
                    let abc = raster.metrics(byte)?;
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
                    raster.print([x + spacing - abc[0], y + spacing], &[byte])?;
                    x = (x + abc[1] + 2 * spacing + 3) & !3;
                    x += es;
                }
            } else {
                if p.flags & 2 != 0 {
                    y -= line_height * (p.text.iter().filter(|&&b| b == b'\n').count() as i32 + 1)
                        / 2;
                }
                for line in p.text.split(|&b| b == b'\n') {
                    let cursor = if p.flags & 1 != 0 {
                        x - raster.width(line)? / 2
                    } else {
                        x
                    };
                    raster.print([cursor, y], line)?;
                    y += (p.line_skip * line_height as f32) as i32;
                }
            }
            let canvas = raster.finish()?;
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
#[cfg(any(target_os = "windows", test))]
fn font_face_matches(requested: &[u8], selected: &[u8]) -> bool {
    let requested = if requested.is_empty() {
        b"Arial".as_slice()
    } else {
        requested
    };
    requested.eq_ignore_ascii_case(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requested_font_face_must_match_without_silent_substitution() {
        assert!(font_face_matches(b"", b"Arial"));
        assert!(font_face_matches(b"arial", b"ARIAL"));
        assert!(font_face_matches(b"Times New Roman", b"times new roman"));
        assert!(!font_face_matches(b"Missing Face", b"Arial"));
        assert!(!font_face_matches(b"Arial", b"Arial Black"));
        assert!(!font_face_matches(b"Arial", b""));
    }
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
