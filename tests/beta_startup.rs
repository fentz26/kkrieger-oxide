use kkrieger_oxide::{
    game::{Parameters, root},
    kx::{Document, Layout},
    scene::exec::Execution,
};
#[test]
fn released_startup_origin_and_switches_match_native_sequence() {
    let bytes = include_bytes!("fixtures/beta-startup.bin");
    assert_eq!(bytes.len(), 1284);
    let mut words = bytes[..1028]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(32));
    for case in 0..32 {
        assert_eq!(words.next(), Some(case + 1));
        assert_eq!(words.next(), Some(case));
        let pose = std::array::from_fn::<_, 4, _>(|_| f32::from_bits(words.next().unwrap()));
        assert_eq!(words.next(), Some(0));
        assert_eq!(words.next(), Some(0));
        let parameters = Parameters {
            start_position: [case as f32, 1.75, -21., 1.],
            ..Default::default()
        };
        assert_eq!(
            root::initial_player_position(&parameters, Layout::Beta2004),
            pose
        );
        assert_eq!(
            root::initial_player_position(&parameters, Layout::Legacy3383),
            parameters.start_position
        );
    }
    assert!(words.next().is_none());
    assert_eq!(
        root::switches_for_layout(Layout::Beta2004).as_slice(),
        &bytes[1028..]
    );
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.layout = Layout::Beta2004;
    assert_eq!(
        Execution::new_player(&doc).switches.as_slice(),
        &bytes[1028..]
    );
}
