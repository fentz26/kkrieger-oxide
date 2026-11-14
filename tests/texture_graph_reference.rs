use kkrieger_oxide::{
    kx::{Document, Layout},
    texture,
};

fn verify(data: &[u8], layout: Layout, reference: &[u8], expected_count: usize) {
    let doc = Document::parse_layout(data, layout).unwrap();
    let mut pos = 0;
    fn word(b: &[u8], p: &mut usize) -> u32 {
        let w = u32::from_le_bytes(b[*p..*p + 4].try_into().unwrap());
        *p += 4;
        w
    }
    let count = word(reference, &mut pos) as usize;
    assert_eq!(count, expected_count);
    let entries: Vec<_> = (0..count)
        .map(|_| std::array::from_fn::<_, 8, _>(|_| word(reference, &mut pos)))
        .collect();
    assert_eq!(pos, reference.len());
    let roots: Vec<_> = entries.iter().map(|e| e[0] as usize).collect();
    let images = texture::generate_many(&doc, &roots, 0).unwrap();
    for (image, expected) in images.iter().zip(entries) {
        let id = expected[0];
        assert_eq!(
            [
                image.width as u32,
                image.height as u32,
                image.format,
                image.mip_count,
                image.mip_threshold
            ],
            expected[1..6],
            "op {id} metadata"
        );
        let actual = image
            .pixels
            .iter()
            .flatten()
            .fold(0xcbf29ce484222325_u64, |h, &w| {
                w.to_le_bytes()
                    .iter()
                    .fold(h, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
            });
        let hash = expected[6] as u64 | ((expected[7] as u64) << 32);
        assert_eq!(actual, hash, "op {id} full-resolution composed pixels");
    }
}

#[test]
#[ignore = "full-resolution development regression; use --release"]
fn development_composed_non_font_textures_match_original_cpp() {
    verify(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
        include_bytes!("fixtures/texture-graph3383.bin"),
        102,
    );
}

#[test]
#[ignore = "full-resolution beta regression requires local original release data; use --release"]
fn beta_composed_non_font_textures_match_original_cpp() {
    verify(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
        &std::fs::read(".reference/texture-graph-beta.bin").unwrap(),
        95,
    );
}
