use kkrieger_oxide::synth::{
    allocation::Allocator,
    controls::{Changes, Channel, Speech, State},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn byte(&mut self) -> u8 {
        let n = self.0[0];
        self.0 = &self.0[1..];
        n
    }
}
#[test]
fn original_controller_program_and_note_off_effects_match_512_states() {
    let mut r = Reader(include_bytes!("fixtures/synth-controls.bin"));
    assert_eq!(r.word(), 512);
    for case in 0..512 {
        let kind = r.word();
        let channel = r.word() as u8;
        let sample_rate = r.word();
        let first = r.word() as u8;
        let second = r.word() as u8;
        let channels = std::array::from_fn(|_| Channel {
            program: r.byte(),
            controls: std::array::from_fn(|_| r.byte()),
        });
        let mut state = State {
            channels,
            sample_rate,
        };
        let mut allocator = Allocator {
            channels: std::array::from_fn(|_| r.word() as i32),
            ..Allocator::default()
        };
        let notes = std::array::from_fn(|_| r.word() as i32);
        let gates = std::array::from_fn(|_| r.word());
        let changes = match kind {
            0 => state
                .control(channel, first, second, &mut allocator)
                .unwrap(),
            1 => {
                state.program(channel, first, &mut allocator).unwrap();
                Changes::default()
            }
            _ => state
                .note_off(channel, first, &allocator, &notes, &gates)
                .unwrap(),
        };
        assert_eq!(state.sample_rate, r.word(), "case {case} sample rate");
        for value in allocator.channels {
            assert_eq!(value as u32, r.word(), "case {case} map");
        }
        for info in state.channels {
            assert_eq!(info.program, r.byte());
            for value in info.controls {
                assert_eq!(value, r.byte(), "case {case} controller");
            }
        }
        assert_eq!(changes.reset, r.word(), "case {case} resets");
        assert_eq!(changes.release, r.word(), "case {case} releases");
        let (kind, index, value) = match changes.speech {
            None => (0, 0, 0),
            Some(Speech::Release) => (2, 0, 0),
            Some(Speech::Controller { index, value }) => (1, u32::from(index), u32::from(value)),
        };
        assert_eq!(
            [kind, index, value],
            [r.word(), r.word(), r.word()],
            "case {case} speech callback"
        );
    }
    assert!(r.0.is_empty());
}
#[test]
fn initial_controller_defaults_and_invalid_channels_are_bounded() {
    let mut state = State::new(44100);
    let mut allocator = Allocator::default();
    assert!(
        state
            .channels
            .iter()
            .all(|c| c.program == 0 && c.controls == [0, 0, 0, 0, 0, 0, 127])
    );
    assert!(state.program(16, 0, &mut allocator).is_err());
    assert!(state.control(16, 120, 0, &mut allocator).is_err());
    assert!(
        state
            .note_off(16, 60, &allocator, &[0; 32], &[0; 32])
            .is_err()
    );
    assert_eq!(allocator.channels, [-1; 32]);
}
