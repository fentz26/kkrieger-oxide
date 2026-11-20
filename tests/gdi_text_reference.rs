#![cfg(target_os = "windows")]
use kkrieger_oxide::bitmap::{Bitmap, FontBook, Text, color64, fade64};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let word = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        word
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
#[test]
#[ignore = "requires tools/verify_gdi_font.py capture on these Windows fonts"]
fn atlas_metrics_and_pixels_match_published_gdi_routines() {
    let bytes = std::fs::read(".reference/gdi-font.bin").unwrap();
    let mut r = Reader(&bytes);
    assert_eq!(r.word(), 24);
    let mut fonts = FontBook::new();
    for case in 0..24 {
        let font: &[u8] = match r.word() {
            0 => b"Arial",
            1 => b"Times New Roman",
            _ => panic!("font"),
        };
        let position = [r.float(), r.float()];
        let width = r.float();
        let height = r.float();
        let external_space = r.word() as i32;
        let internal_space = r.float();
        let mut bitmap = Bitmap::flat(7, 6, 0xff000000, 0);
        bitmap
            .text(
                Text {
                    position,
                    width,
                    height,
                    color: 0xffffffff,
                    flags: 0x10,
                    external_space,
                    internal_space,
                    line_skip: 1.,
                    text: b" -~",
                    font,
                    blob: &[],
                },
                &mut fonts,
            )
            .unwrap();
        let page = bitmap.font_page.as_ref().unwrap();
        assert_eq!(u32::from(page.line_height), r.word(), "case {case} height");
        for (index, letter) in page.letters.iter().enumerate() {
            for actual in letter.origin {
                assert_eq!(u32::from(actual), r.word(), "case {case}/{index} origin");
            }
            for actual in letter.abc {
                assert_eq!(
                    i32::from(actual),
                    r.word() as i32,
                    "case {case}/{index} ABC"
                );
            }
            for actual in letter
                .uv
                .into_iter()
                .chain([letter.pre_space, letter.width])
            {
                assert_eq!(
                    actual.to_bits(),
                    r.word(),
                    "case {case}/{index} metric {actual}"
                );
            }
        }
        for (index, actual) in bitmap.pixels.iter().enumerate() {
            let coverage = r.0[0];
            r.0 = &r.0[1..];
            let expected = fade64(
                color64(0xff000000),
                color64(0xffffffff),
                i32::from(coverage) * 257,
            );
            assert_eq!(*actual, expected, "case {case} pixel {index}");
        }
    }
    assert!(r.0.is_empty());
}

#[test]
#[ignore = "requires tools/verify_beta_gdi_font.py capture on these Windows fonts"]
fn released_beta_text_matches_native_initializer_pixels_and_uvs() {
    use kkrieger_oxide::kx::Layout;
    let bytes = std::fs::read(".reference/beta-gdi-font.bin").unwrap();
    let mut r = Reader(&bytes);
    assert_eq!(r.word(), 24);
    let mut fonts = FontBook::new();
    for case in 0..24 {
        let side = r.word() as usize;
        let font: &[u8] = match r.word() {
            0 => b"Arial",
            1 => b"Times New Roman",
            _ => panic!("font"),
        };
        let position = [r.float(), r.float()];
        let width = r.float();
        let height = r.float();
        let color = r.word();
        let flags = r.word();
        let external_space = r.word() as i32;
        let internal_space = r.float();
        let n = r.word() as usize;
        let text = &r.0[..n];
        r.0 = &r.0[n..];
        let mut bitmap = Bitmap::flat(side.ilog2() as i32, side.ilog2() as i32, 0xff102030, 0);
        bitmap
            .text_for_layout(
                Text {
                    position,
                    width,
                    height,
                    color,
                    flags,
                    external_space,
                    internal_space,
                    line_skip: 1.,
                    text,
                    font,
                    blob: &[],
                },
                &mut fonts,
                Layout::Beta2004,
            )
            .unwrap();
        let line_height = r.word();
        if let Some(page) = &bitmap.font_page {
            assert_eq!(
                u32::from(page.line_height),
                line_height,
                "case{case} height"
            );
            for (glyph, letter) in page.letters.iter().enumerate() {
                for value in letter.uv {
                    assert_eq!(value.to_bits(), r.word(), "case{case}/{glyph} UV");
                }
            }
        } else {
            assert_eq!(flags & 0x70, 0);
            assert!(r.0[..4096].iter().all(|&v| v == 0));
            r.0 = &r.0[4096..];
        }
        for (index, pixel) in bitmap.pixels.iter().enumerate() {
            for value in pixel {
                let expected = u16::from_le_bytes(r.0[..2].try_into().unwrap());
                r.0 = &r.0[2..];
                assert_eq!(*value, expected, "case{case} pixel{index}");
            }
        }
    }
    assert!(r.0.is_empty());
}
