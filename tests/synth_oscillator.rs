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

impl Reader<'_> {
    fn read_oscillator(&mut self) -> Oscillator {
        Oscillator {
            mode: self.word(),
            ring: self.word(),
            counter: self.word(),
            frequency: self.word(),
            breakpoint: self.word(),
            noise_frequency: self.float(),
            noise_resonance: self.float(),
            seed: self.word(),
            gain: self.float(),
            oversampled_gain: self.float(),
            temporary: self.word(),
            noise_low: self.float(),
            noise_band: self.float(),
            note: self.float(),
            pitch: self.float(),
        }
    }
}
#[test]
fn original_all_waveforms_ring_modes_and_384_persistent_render_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-oscillator-render.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 3);
    let mut coverage = [0u8; 2];
    for case in 0..128 {
        let count = r.word() as usize;
        let mut oscillator = r.read_oscillator();
        for block in 0..3 {
            let mut destination: Vec<_> = (0..count).map(|_| r.float()).collect();
            let a: Vec<[f32; 2]> = (0..count)
                .map(|_| std::array::from_fn(|_| r.float()))
                .collect();
            let b: Vec<[f32; 2]> = (0..count)
                .map(|_| std::array::from_fn(|_| r.float()))
                .collect();
            if matches!(oscillator.mode, 1 | 2) {
                let mut counter = oscillator.counter;
                for _ in 0..count {
                    let previous = counter.wrapping_sub(oscillator.frequency);
                    let transition = (u32::from(previous < oscillator.breakpoint) << 2)
                        | (u32::from(counter < oscillator.frequency) << 1)
                        | u32::from(counter < oscillator.breakpoint);
                    coverage[oscillator.mode as usize - 1] |= 1 << transition;
                    counter = counter.wrapping_add(oscillator.frequency);
                }
            }
            oscillator.render(&mut destination, &a, &b).unwrap();
            r.oscillator(&oscillator);
            for (sample, output) in destination.into_iter().enumerate() {
                let expected = r.float();
                // Unicorn FPREM loses the zero sign for exact negative FM
                // cycles. The separate native-x86 sine fixture verifies it.
                assert!(
                    output.to_bits() == expected.to_bits()
                        || oscillator.mode == 5 && output == 0. && expected == 0.,
                    "case {case} block {block} sample {sample}: {output} != {expected}"
                );
            }
        }
    }
    assert_eq!(coverage, [0xbd; 2]);
    assert!(r.0.is_empty());
}
#[test]
fn oscillator_auxiliary_bounds_are_checked_before_mutation_and_empty_frames_are_safe() {
    let mut oscillator = Oscillator {
        mode: 6,
        counter: 17,
        ..Default::default()
    };
    let mut buffer = [1., 2.];
    assert!(oscillator.render(&mut buffer, &[], &[]).is_err());
    assert_eq!(buffer, [1., 2.]);
    assert_eq!(oscillator.counter, 17);
    assert!(oscillator.render(&mut [], &[], &[]).is_ok());
}
