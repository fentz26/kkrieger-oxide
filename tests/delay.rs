use kkrieger_oxide::scene::delay::{Parameters, State};

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}

#[test]
fn persistent_delay_matches_original_handler() {
    let mut r = Reader(include_bytes!("fixtures/delay.bin"));
    let cases = r.word();
    let steps = r.word();
    for case in 0..cases {
        let p = Parameters {
            flags: r.word(),
            threshold: r.float(),
            hysteresis: r.float(),
            durations: std::array::from_fn(|_| r.float()),
        };
        let mut state = State::default();
        for step in 0..steps {
            let switch = r.word() as u8;
            let current = r.word() as i32;
            let last = r.word() as i32;
            let mut variables: [[f32; 4]; 4] =
                std::array::from_fn(|_| std::array::from_fn(|_| r.float()));
            state.advance(p, switch, &mut variables, current, last);
            assert_eq!(state.phase, r.word() as i32, "{case}/{step}");
            assert_eq!(state.time.to_bits(), r.word(), "{case}/{step}");
            for value in variables.into_iter().flatten() {
                assert_eq!(value.to_bits(), r.word(), "{case}/{step}");
            }
        }
    }
    assert!(r.0.is_empty());
}
