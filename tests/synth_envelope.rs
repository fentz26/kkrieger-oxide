use kkrieger_oxide::synth::envelope::Envelope;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn envelope(&mut self) -> Envelope {
        Envelope {
            output: self.float(),
            state: self.word(),
            value: self.float(),
            attack_delta: self.float(),
            decay_factor: self.float(),
            sustain_level: self.float(),
            sustain_factor: self.float(),
            release_factor: self.float(),
            gain: self.float(),
        }
    }
    fn check(&mut self, e: &Envelope) {
        assert_eq!(e.output.to_bits(), self.word());
        assert_eq!(e.state, self.word());
        for v in [
            e.value,
            e.attack_delta,
            e.decay_factor,
            e.sustain_level,
            e.sustain_factor,
            e.release_factor,
            e.gain,
        ] {
            assert_eq!(v.to_bits(), self.word(), "{v}");
        }
    }
}
#[test]
fn original_x87_envelope_transitions_and_outputs_match_for_8192_ticks() {
    let mut r = Reader(include_bytes!("fixtures/synth-envelope.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 64);
    for _ in 0..128 {
        let mut e = r.envelope();
        for _ in 0..64 {
            let gate = r.word() != 0;
            e.tick(gate).unwrap();
            r.check(&e);
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn init_only_clears_state_and_invalid_state_is_rejected() {
    let mut e = Envelope {
        state: 3,
        value: 64.,
        output: 16.,
        gain: 0.25,
        ..Default::default()
    };
    e.init();
    assert_eq!(e.state, 0);
    assert_eq!(e.value, 64.);
    assert_eq!(e.output, 16.);
    assert_eq!(e.gain, 0.25);
    e.state = 5;
    assert!(e.tick(true).is_err());
    assert_eq!(e.value, 64.);
}
