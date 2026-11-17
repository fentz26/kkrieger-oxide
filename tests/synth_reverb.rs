use kkrieger_oxide::synth::{
    rate::Rate,
    reverb::{LENGTHS, Parameters, Reverb},
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
    fn coefficients(&mut self, r: &Reverb, case: usize) {
        for (i, value) in r
            .gains
            .into_iter()
            .chain([r.input_gain, r.damping, r.low_cut])
            .enumerate()
        {
            assert_eq!(value.to_bits(), self.word(), "case {case} coefficient {i}");
        }
    }
    fn state(&mut self, r: &Reverb, case: usize, block: usize) {
        self.coefficients(r, case);
        for (i, value) in r
            .positions
            .into_iter()
            .chain(r.lowpass.map(f32::to_bits))
            .chain(r.highpass.map(f32::to_bits))
            .enumerate()
        {
            assert_eq!(value, self.word(), "case {case} block {block} state {i}");
        }
    }
}
#[test]
fn original_reverb_coefficients_filters_rings_and_256_stereo_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-reverb.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 4);
    let size = r.word();
    assert_eq!(size as usize, 31 + LENGTHS.iter().sum::<usize>());
    for case in 0..64 {
        let count = r.word() as usize;
        let rate = Rate::new(r.word()).unwrap();
        let p = Parameters {
            time: r.float(),
            high_cut: r.float(),
            low_cut: r.float(),
            volume: r.float(),
        };
        let mut effect = Reverb::default();
        effect.init();
        effect.set(p, &rate);
        r.coefficients(&effect, case);
        let mut seed = r.word();
        for value in &mut effect.history {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *value = (((seed >> 16) % 1024) as i32 - 512) as f32 / 512.;
        }
        effect.positions = std::array::from_fn(|i| (LENGTHS[i] - 1) as u32);
        for block in 0..4 {
            if block == 2 {
                effect.reset();
            }
            let mut mix: Vec<_> = (0..count).map(|_| [r.float(), r.float()]).collect();
            let input: Vec<_> = (0..count).map(|_| r.float()).collect();
            effect.process(&mut mix, &input).unwrap();
            r.state(&effect, case, block);
            for (i, value) in mix.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {i}"
                );
            }
        }
        let mut expected = vec![0; effect.history.len()];
        let n = r.word();
        for _ in 0..n {
            let i = r.word() as usize;
            expected[i] = r.word();
        }
        for (i, (value, expected)) in effect.history.iter().zip(expected).enumerate() {
            assert_eq!(value.to_bits(), expected, "case {case} history {i}");
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn reset_preserves_coefficients_and_short_input_does_not_modify_audio_or_state() {
    let mut r = Reverb::default();
    r.set(
        Parameters {
            time: 64.,
            high_cut: 80.,
            low_cut: 32.,
            volume: 100.,
        },
        &Rate::new(44100).unwrap(),
    );
    let gains = r.gains;
    let setup = [r.input_gain, r.damping, r.low_cut];
    r.positions[0] = 7;
    r.lowpass[0] = 1.;
    r.highpass[0] = 2.;
    r.history[0] = 3.;
    let mut audio = [[4., 5.]; 2];
    assert!(r.process(&mut audio, &[]).is_err());
    assert_eq!(audio, [[4., 5.]; 2]);
    assert_eq!(r.positions[0], 7);
    assert_eq!(r.history[0], 3.);
    r.reset();
    assert_eq!(r.gains, gains);
    assert_eq!([r.input_gain, r.damping, r.low_cut], setup);
    assert_eq!(r.positions, [0; 12]);
    assert_eq!(r.lowpass, [0.; 8]);
    assert_eq!(r.highpass, [0.; 2]);
    assert!(r.history.iter().all(|&v| v == 0.));
    r.process(&mut [], &[]).unwrap();
    r.init();
    assert_eq!(r.gains, [0.; 6]);
    assert_eq!([r.input_gain, r.damping, r.low_cut], [0.; 3]);
}

#[test]
fn native_x87_underflow_samples_and_tail_recovery_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-reverb-underflow.bin"));
    assert_eq!(r.word(), 64);
    let words = r.word() as usize;
    for case in 0..64 {
        let mut effect = Reverb {
            gains: std::array::from_fn(|_| r.float()),
            input_gain: r.float(),
            damping: r.float(),
            low_cut: r.float(),
            ..Reverb::default()
        };
        for line in [4, 5, 10, 11] {
            let offset = LENGTHS[..line].iter().sum::<usize>();
            effect.history[offset] = r.float();
        }
        let n = r.word() as usize;
        let mut output = vec![[0.; 2]; n];
        effect.process(&mut output, &vec![0.; n]).unwrap();
        for (sample, value) in output.into_iter().flatten().enumerate() {
            assert_eq!(
                value.to_bits(),
                r.word(),
                "case {case} tiny sample {sample}"
            );
        }
        r.state(&effect, case, 0);
        let mut expected = vec![0; words - 31];
        let count = r.word();
        for _ in 0..count {
            let i = r.word() as usize;
            expected[i] = r.word();
        }
        for (i, (value, expected)) in effect.history.iter().zip(expected).enumerate() {
            assert_eq!(
                value.to_bits(),
                expected,
                "case {case} recovery history {i}"
            );
        }
    }
    assert!(r.0.is_empty());
}
