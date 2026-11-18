#[test]
fn packed_project_decodes_to_the_exact_original_document() {
    let expected = include_bytes!("../assets/kkrieger3383.kx");
    let actual = kkrieger_oxide::embedded::data().unwrap();
    assert_eq!(actual, expected);
    let document = kkrieger_oxide::kx::Document::parse(actual).unwrap();
    assert_eq!(document.operators.len(), 5174);
    assert!(std::ptr::eq(
        actual,
        kkrieger_oxide::embedded::data().unwrap()
    ));
}

#[allow(dead_code)]
#[path = "../build.rs"]
mod encoder;

#[test]
fn encoder_finds_short_matches_and_chooses_a_minimal_token_parse() {
    assert_eq!(encoder::pack(b""), b"");
    assert_eq!(encoder::pack(b"abcdef"), b"\x05abcdef");
    // One literal followed by an overlapping 129-byte match is five bytes.
    assert_eq!(encoder::pack(&[b'a'; 130]), [0, b'a', 0xfe, 1, 0]);
    let project = include_bytes!("../assets/kkrieger3383.kx");
    let packed = encoder::pack(project);
    assert!(
        packed.len() < 50000,
        "packed project: {} bytes",
        packed.len()
    );
    assert_eq!(
        packed,
        encoder::pack(project),
        "packing must be deterministic"
    );
}
