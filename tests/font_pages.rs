use kkrieger_oxide::{kx::Document, texture};
#[test]
fn original_font_page_side_effect_survives_consumed_atlas_intermediate() {
    let mut blob = vec![0u8; 8 + 2560 + 4];
    blob[..8].copy_from_slice(&[3, 1, 2, 0, 2, 0, 1, 0]);
    blob[8 + 65 * 10 + 6] = 1;
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.operators.truncate(4);
    let flat = d.classes.iter().position(|c| c.id == 0x21).unwrap();
    let text = d.classes.iter().position(|c| c.id == 0x31).unwrap();
    let merge = d.classes.iter().position(|c| c.id == 0x24).unwrap();
    for op in &mut d.operators {
        op.class = flat;
        op.inputs.clear();
        op.links.clear();
        op.animation = &[1];
        op.parameters = vec![1, 1, 0];
        op.blob = &[];
    }
    d.operators[1].class = text;
    d.operators[1].inputs = vec![0];
    d.operators[1].parameters = vec![
        0,
        0,
        0.5f32.to_bits(),
        0.5f32.to_bits(),
        0xffffffff,
        0x90,
        0,
        0,
        1f32.to_bits(),
    ];
    d.operators[1].strings = vec![b"A", b"Arial"];
    d.operators[1].blob = &blob;
    d.operators[3].class = merge;
    d.operators[3].inputs = vec![2, 1];
    d.operators[3].parameters = vec![0];
    let generated = texture::generate_many_with_fonts(&d, &[3], 0).unwrap();
    assert_eq!(generated.images.len(), 1);
    assert!(generated.images[0].font_page.is_none());
    assert_eq!(generated.font_pages.len(), 1);
    let page = &generated.font_pages[&1];
    assert_eq!(page.page, 1);
    assert_eq!(page.letters[65].width, 0.125);
}

#[test]
#[ignore = "generates the authored font atlas bitmaps"]
fn authored_font_atlases_register_original_pages() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let roots: Vec<_> = d
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| d.classes[op.class].id == 0x31 && op.parameters[5] & 0x70 != 0)
        .map(|(i, _)| i)
        .collect();
    assert!(!roots.is_empty());
    let generated = texture::generate_many_with_fonts(&d, &roots, 0).unwrap();
    assert_eq!(generated.images.len(), roots.len());
    assert!(!generated.font_pages.is_empty());
    for page in generated.font_pages.values() {
        assert!(page.letters.iter().any(|letter| letter.width > 0.));
    }
    eprintln!(
        "authored atlas roots={roots:?} pages={:?}",
        generated.font_pages.keys().collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "requires installed original Arial; see BUILDING.md"]
fn authored_menu_font_packs_real_glyphs_with_windows_line_metrics() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let roots: Vec<_> = d
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| d.classes[op.class].id == 0x31 && (op.parameters[5] & 0x70) == 0x10)
        .map(|(i, _)| i)
        .collect();
    assert!(!roots.is_empty());
    let generated = texture::generate_many_with_fonts(&d, &roots, 0).unwrap();
    let page = &generated.font_pages[&1];
    assert!(
        page.line_height < 1024,
        "inflated font line height: {}",
        page.line_height
    );
    let letter = page.letters[b'a' as usize];
    assert!(letter.abc[1] > 0, "menu glyph was not packed");
    assert!(letter.uv[3] > letter.uv[1] && letter.uv[3] <= 1.0);
    assert!(letter.width > 0.0);
}

#[test]
#[ignore = "requires installed original Arial; see BUILDING.md"]
fn released_beta_font_layout_reaches_bitmap_and_texture_generation() {
    use kkrieger_oxide::{
        bitmap::{Bitmap, FontBook, Text},
        kx::Layout,
    };
    let mut fonts = FontBook::new();
    let p = Text {
        position: [0., 0.],
        width: 0.1,
        height: 0.,
        color: 0xffffffff,
        flags: 0x10,
        external_space: 1,
        internal_space: f32::from_bits(163),
        line_skip: 1.,
        text: b"a-c",
        font: b"Arial",
        blob: &[],
    };
    let mut bitmap = Bitmap::flat(5, 5, 0xff000000, 0);
    bitmap
        .text_for_layout(p, &mut fonts, Layout::Beta2004)
        .unwrap();
    let page = bitmap.font_page.as_ref().unwrap();
    assert_eq!(page.letters[b'a' as usize].uv[0], 4. / 128.);
    assert_eq!(page.letters[b'a' as usize].uv[1], 4. / 128.);
    assert!(page.letters[b'b' as usize].uv[2] > page.letters[b'b' as usize].uv[0]);
    assert_eq!(page.letters[b'c' as usize].uv, [0.; 4]);
    assert!(
        page.letters
            .iter()
            .all(|l| l.width == 0. && l.pre_space == 0. && l.abc == [0; 3])
    );
    assert!(bitmap.pixels.iter().any(|p| p[0] > 0));

    // The released entry has no operator/cache argument; a later-source blob
    // must not replace its native metrics, even when the cache flag is set.
    let mut blob = vec![7; 8 + 2560 + 32 * 32];
    blob[..8].copy_from_slice(&[3, 1, 32, 0, 32, 0, 0xff, 0x03]);
    let mut cached = Bitmap::flat(5, 5, 0xff000000, 0);
    cached
        .text_for_layout(
            Text {
                flags: 0x90,
                blob: &blob,
                ..p
            },
            &mut fonts,
            Layout::Beta2004,
        )
        .unwrap();
    assert_eq!(cached.pixels, bitmap.pixels);
    assert_eq!(
        cached.font_page.as_ref().unwrap().letters[b'a' as usize].uv,
        page.letters[b'a' as usize].uv
    );

    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.layout = Layout::Beta2004;
    d.operators.truncate(2);
    let flat = d.classes.iter().position(|c| c.id == 0x21).unwrap();
    let text = d.classes.iter().position(|c| c.id == 0x31).unwrap();
    d.operators[0].class = flat;
    d.operators[0].inputs.clear();
    d.operators[0].links.clear();
    d.operators[0].parameters = vec![5, 5, 0xff000000];
    d.operators[1].class = text;
    d.operators[1].inputs = vec![0];
    d.operators[1].links.clear();
    d.operators[1].parameters = vec![0, 0, 0.1f32.to_bits(), 0, 0xffffffff, 0x10, 1, 163];
    d.operators[1].strings = vec![b"a-c", b"Arial"];
    d.operators[1].blob = &[];
    let generated = texture::generate_many_with_fonts(&d, &[1], 0).unwrap();
    assert_eq!(
        generated.font_pages[&1].letters[b'a' as usize].uv,
        page.letters[b'a' as usize].uv
    );
    d.operators[1].parameters[7] = 0.02f32.to_bits();
    let spaced = texture::generate_many_with_fonts(&d, &[1], 0).unwrap();
    assert_eq!(
        spaced.font_pages[&1].letters[b'a' as usize].uv[2],
        page.letters[b'a' as usize].uv[2] + 4. / 128.
    );
}
