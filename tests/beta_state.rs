use kkrieger_oxide::{kx::Layout, scene::menu};
#[test]
fn released_state_switch_writes_and_key_consumption_match_native() {
    let bytes = include_bytes!("fixtures/beta-state.bin");
    assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 480);
    assert_eq!(bytes.len(), 4 + 480 * 288);
    for row in bytes[4..].as_chunks::<288>().0 {
        let words: Vec<_> = row[..32]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        let mut switches =
            std::array::from_fn(|index| words[0].wrapping_add(17 * index as u32) as u8);
        let mut key = words[6];
        menu::state_for_layout(
            &mut switches,
            &mut key,
            words[1..6].try_into().unwrap(),
            Layout::Beta2004,
        )
        .unwrap();
        assert_eq!(key, words[7], "case {}", words[0]);
        assert_eq!(switches.as_slice(), &row[32..], "case {}", words[0]);
    }
}
#[test]
fn development_state_retains_mouse_activation() {
    let mut switches = [0; 256];
    let mut key = kkrieger_oxide::game::input::MOUSE_LEFT;
    menu::state_for_layout(&mut switches, &mut key, &[1, 5, 2, 0, 0], Layout::Current).unwrap();
    assert_eq!(switches[1], 5);
    assert_eq!(key, 0);
}
