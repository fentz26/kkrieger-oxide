use kkrieger_oxide::synth::{
    oscillator::{Oscillator, Parameters},
    rate::Rate,
};
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
    fn rate(&mut self, r: &Rate) {
        for n in [
            r.oscillator_base.to_bits(),
            r.linear_frequency.to_bits(),
            r.boost_sine.to_bits(),
            r.boost_cosine.to_bits(),
            r.frame_size,
        ] {
            assert_eq!(n, self.word());
        }
    }
    fn oscillator(&mut self, o: &Oscillator) {
        for n in [
            o.mode,
            o.ring,
            o.counter,
            o.frequency,
            o.breakpoint,
            o.noise_frequency.to_bits(),
            o.noise_resonance.to_bits(),
            o.seed,
            o.gain.to_bits(),
            o.oversampled_gain.to_bits(),
            o.temporary,
            o.noise_low.to_bits(),
            o.noise_band.to_bits(),
            o.note.to_bits(),
            o.pitch.to_bits(),
        ] {
            assert_eq!(n, self.word());
        }
    }
}
#[test]
fn original_x87_sample_rate_oscillator_parameters_and_pitch_changes_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-oscillator-set.bin"));
    assert_eq!(r.word(), 256);
    for _ in 0..256 {
        let samples = r.word();
        let note = r.float();
        let counter = r.word();
        let seed = r.word();
        let p = Parameters {
            mode: r.float(),
            ring: r.float(),
            pitch: r.float(),
            detune: r.float(),
            color: r.float(),
            gain: r.float(),
        };
        let rate = Rate::new(samples).unwrap();
        r.rate(&rate);
        let mut oscillator = Oscillator {
            note,
            counter,
            seed,
            ..Default::default()
        };
        oscillator.set(p, &rate);
        r.oscillator(&oscillator);
        for _ in 0..2 {
            oscillator.note = r.float();
            oscillator.change_pitch(&rate);
            r.oscillator(&oscillator);
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn oscillator_init_preserves_patch_and_note_and_invalid_sample_rates_are_rejected() {
    let mut oscillator = Oscillator {
        note: 60.,
        pitch: 3.,
        gain: 1.,
        counter: 17,
        noise_low: 19.,
        noise_band: 23.,
        ..Default::default()
    };
    oscillator.init(0x12345678);
    assert_eq!(oscillator.counter, 0);
    assert_eq!(oscillator.seed, 0x12345678);
    assert_eq!(oscillator.noise_low, 0.);
    assert_eq!(oscillator.noise_band, 0.);
    assert_eq!(oscillator.note, 60.);
    assert_eq!(oscillator.pitch, 3.);
    assert_eq!(oscillator.gain, 1.);
    assert!(Rate::new(0).is_err());
    assert!(Rate::new(0x80000000).is_err());
    assert_eq!(Rate::new(44100).unwrap().frame_size, 128);
}
