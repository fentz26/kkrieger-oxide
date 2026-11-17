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
