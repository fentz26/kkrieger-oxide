use kkrieger_oxide::synth::lfo::{Lfo, Parameters};
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
    fn check(&mut self, lfo: &Lfo) {
        assert_eq!(lfo.output.to_bits(), self.word(), "output {}", lfo.output);
        for value in [
            lfo.mode,
            lfo.sync,
            lfo.envelope,
            lfo.frequency,
            lfo.counter,
            lfo.phase,
            lfo.gain.to_bits(),
            lfo.dc.to_bits(),
            lfo.seed,
            lfo.last,
        ] {
            assert_eq!(value, self.word());
        }
    }
}
#[test]
fn original_x87_lfo_conversion_key_sync_and_8192_ticks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-lfo.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 64);
    for _ in 0..128 {
        let p = Parameters {
            mode: r.float(),
            sync: r.float(),
            envelope: r.float(),
            rate: r.float(),
            phase: r.float(),
            polarity: r.float(),
            amplitude: r.float(),
        };
        let mut lfo = Lfo {
            output: 16.,
            counter: r.word(),
            seed: r.word(),
            last: r.word(),
            gain: 17.,
            dc: 17.,
            ..Default::default()
        };
        lfo.set(p);
        r.check(&lfo);
        for _ in 0..64 {
            if r.word() != 0 {
                lfo.key_on();
            }
            lfo.tick().unwrap();
            r.check(&lfo);
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn lfo_init_keeps_parameters_and_invalid_modes_do_not_advance() {
    let mut lfo = Lfo {
        counter: 7,
        last: 11,
        gain: 17.,
        phase: 19,
        ..Default::default()
    };
    lfo.init(0x12345678);
    assert_eq!(lfo.counter, 0);
    assert_eq!(lfo.last, 0);
    assert_eq!(lfo.seed, 0x12345678);
    assert_eq!(lfo.gain, 17.);
    assert_eq!(lfo.phase, 19);
    lfo.mode = 8;
    assert!(lfo.tick().is_err());
    assert_eq!(lfo.counter, 0);
}

#[test]
fn original_fast_sine_retains_negative_fm_range_behavior() {
    let mut r = Reader(include_bytes!("fixtures/synth-fast-sine.bin"));
    assert_eq!(r.word(), 2048);
    for _ in 0..2048 {
        let input = r.float();
        let output = kkrieger_oxide::synth::math::fast_sine(input);
        assert_eq!(output.to_bits(), r.word(), "input {input}: {output}");
    }
    assert!(r.0.is_empty());
}
