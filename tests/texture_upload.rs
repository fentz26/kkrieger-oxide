use kkrieger_oxide::{
    bitmap::Bitmap,
    texture_upload::{Encoding, Pyramid},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn half(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.0[..2].try_into().unwrap());
        self.0 = &self.0[2..];
        v
    }
}
#[test]
fn original_player_texture_conversion_and_mips_match_cpp() {
    let mut r = Reader(include_bytes!("fixtures/texture-upload.bin"));
    assert_eq!(r.word(), 256);
    for case in 0..256 {
        let width = r.word() as usize;
        let height = r.word() as usize;
        let format = r.word();
        let mip_count = r.word();
        let mip_threshold = r.word();
        let pixels = (0..width * height)
            .map(|_| std::array::from_fn(|_| r.half()))
            .collect();
        let bitmap = Bitmap {
            width,
            height,
            format,
            mip_count,
            mip_threshold,
            pixels,
            font_page: None,
        };
        let pyramid = Pyramid::from_bitmap(&bitmap).unwrap();
        assert_eq!(
            pyramid.encoding,
            if format == 5 {
                Encoding::Rgba8Snorm
            } else {
                Encoding::Rgba8
            }
        );
        assert_eq!(r.word() as usize, pyramid.allocated_levels, "case {case}");
        assert_eq!(r.word() as usize, pyramid.levels.len(), "case {case}");
        for level in pyramid.levels {
            assert_eq!(r.word() as usize, level.width);
            assert_eq!(r.word() as usize, level.height);
            let n = level.pixels.len();
            assert_eq!(&level.pixels[..], &r.0[..n], "case {case}");
            r.0 = &r.0[n..];
        }
    }
    assert!(r.0.is_empty());
}
