use kkrieger_oxide::synth::compressor::{Compressor, Parameters};
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
    fn parameters(&mut self) -> Parameters {
        Parameters {
            mode: self.float(),
            stereo: self.float(),
            automatic_gain: self.float(),
            lookahead: self.float(),
            threshold: self.float(),
            ratio: self.float(),
            attack: self.float(),
            release: self.float(),
            output_gain: self.float(),
        }
    }
    fn state(&mut self, c: &Compressor, case: usize, block: usize) {
        for (field, value) in [
            c.mode,
            c.old_mode,
            c.input_gain.to_bits(),
            c.ratio.to_bits(),
            c.output_gain.to_bits(),
            c.attack.to_bits(),
            c.release.to_bits(),
            c.lookahead_length,
            c.lookahead_position,
            c.current_gain[0].to_bits(),
            c.current_gain[1].to_bits(),
            c.peak[0].to_bits(),
            c.peak[1].to_bits(),
            c.rms_position,
            c.rms_value[0].to_bits(),
            c.rms_value[1].to_bits(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                value,
                self.word(),
                "case {case} block {block} field {field}"
            );
        }
    }
}
#[test]
fn original_compressor_coefficients_detectors_lookahead_and_384_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-compressor.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 6);
    let mut levels = vec![[0.; 2]; 280];
    for case in 0..64 {
        let count = r.word() as usize;
        let p = r.parameters();
        let mut c = Compressor::default();
        c.init();
        c.set(p).unwrap();
        c.rms_position = 8190;
        r.state(&c, case, 99);
        for block in 0..6 {
            c.set(r.parameters()).unwrap();
            let mut audio: Vec<_> = (0..count).map(|_| [r.float(), r.float()]).collect();
            c.render(&mut audio, &mut levels).unwrap();
            r.state(&c, case, block);
            for (sample, value) in audio.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {sample}"
                );
            }
        }
        let mut expected = vec![0; 5700 * 2 + 8192 * 2];
        let n = r.word();
        for _ in 0..n {
            let i = r.word() as usize;
            expected[i] = r.word();
        }
        for (i, value) in c
            .lookahead
            .iter()
            .flatten()
            .chain(c.rms_history.iter())
            .enumerate()
        {
            assert_eq!(value.to_bits(), expected[i], "case {case} history {i}");
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn mode_changes_reset_detectors_but_preserve_lookahead_and_ring_positions() {
    let p = Parameters {
        mode: 1.,
        stereo: 0.,
        automatic_gain: 1.,
        lookahead: 2.,
        threshold: 90.,
        ratio: 64.,
        attack: 20.,
        release: 64.,
        output_gain: 64.,
    };
    let mut c = Compressor::default();
    c.init();
    c.set(p).unwrap();
    assert_eq!(c.current_gain, [0.; 2]);
    c.lookahead[3] = [4., 5.];
    c.lookahead_position = 3;
    c.rms_position = 8191;
    c.rms_history[7] = 2.;
    c.peak = [3., 4.];
    c.rms_value = [5., 6.];
    c.set(Parameters { mode: 2., ..p }).unwrap();
    assert_eq!(c.current_gain, [1.; 2]);
    assert_eq!(c.peak, [0.; 2]);
    assert_eq!(c.rms_value, [0.; 2]);
    assert!(c.rms_history.iter().all(|&v| v == 0.));
    assert_eq!(c.lookahead[3], [4., 5.]);
    assert_eq!(c.lookahead_position, 3);
    assert_eq!(c.rms_position, 8191);
    let mut audio = [[1., 2.]; 4];
    assert!(c.render(&mut audio, &mut []).is_err());
    assert_eq!(audio, [[1., 2.]; 4]);
    assert_eq!(c.lookahead_position, 3);
    assert!(
        c.set(Parameters {
            lookahead: 1000.,
            ..p
        })
        .is_err()
    );
    assert_eq!(c.mode, 1);
    c.render(&mut [], &mut []).unwrap();
    assert_eq!(c.lookahead_position, 3);
}
