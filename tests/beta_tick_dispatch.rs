use kkrieger_oxide::game::beta_player::{TickDispatch, dispatch_tick};
#[test]
fn released_mode_dispatch_and_tick_counter_match_native_prologue() {
    let mut words = include_bytes!("fixtures/beta-tick-dispatch.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(1024));
    for case in 0..1024 {
        let present = words.next().unwrap();
        let mode = words.next().unwrap() as u8;
        let mut counter = words.next().unwrap() as i32;
        let slices = words.next().unwrap() as i32;
        let result = dispatch_tick(
            present & 1 != 0,
            present & 2 != 0,
            mode,
            &mut counter,
            slices,
        );
        let actual = match result {
            TickDispatch::Idle => 0,
            TickDispatch::Run => 1,
            TickDispatch::Restart => 2,
            TickDispatch::Quit => 3,
        };
        assert_eq!(actual, words.next().unwrap(), "case {case}");
        assert_eq!(counter as u32, words.next().unwrap(), "case {case}");
    }
    assert!(words.next().is_none());
}
