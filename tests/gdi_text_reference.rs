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
