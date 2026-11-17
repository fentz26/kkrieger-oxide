use kkrieger_oxide::synth::{
    delay::{Delay, Parameters},
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
    fn state(&mut self, d: &Delay, case: usize, block: usize) {
        for (field, n) in [
            d.mask,
            d.write_position,
            d.left_offset,
            d.right_offset,
            d.counter,
            d.frequency,
            d.phase,
            d.depth,
            d.feedback.to_bits(),
            d.dry.to_bits(),
            d.effect.to_bits(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(n, self.word(), "case {case} block {block} field {field}");
        }
    }
}
#[test]
fn original_delay_coefficients_stereo_auxiliary_and_384_persistent_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-delay.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 6);
    for case in 0..64 {
        let length = r.word() as usize;
        let rate = Rate::new(r.word()).unwrap();
        let aux = r.word() != 0;
        let count = r.word() as usize;
        let p = Parameters {
            amount: r.float(),
            feedback: r.float(),
            left_length: r.float(),
            right_length: r.float(),
            rate: r.float(),
            depth: r.float(),
            phase: r.float(),
        };
        let mut d = Delay::new(length).unwrap();
        d.init();
        d.set(p, &rate);
        r.state(&d, case, 99);
        let mut seed = r.word();
        for side in 0..2 {
            for value in &mut d.history {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                value[side] = (((seed >> 16) % 1024) as i32 - 512) as f32 / 512.;
            }
        }
        let initial = d.history.clone();
        for block in 0..6 {
            let mut audio: Vec<_> = (0..count).map(|_| [r.float(), r.float()]).collect();
            let source: Vec<_> = (0..count).map(|_| r.float()).collect();
            if aux {
                d.render_auxiliary(&mut audio, &source).unwrap();
            } else {
                d.render_channel(&mut audio);
            }
            r.state(&d, case, block);
            for (sample, value) in audio.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {sample}"
                );
            }
        }
        let changed = length.min(count * 6);
        for side in 0..2 {
            for (i, value) in d.history.iter().enumerate() {
                let expected = if i < changed {
                    r.word()
                } else {
                    initial[i][side].to_bits()
                };
                assert_eq!(
                    value[side].to_bits(),
                    expected,
                    "case {case} history {side} sample {i}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn initialization_clears_history_and_bad_auxiliary_input_does_not_advance() {
    assert!(Delay::new(0).is_err());
    assert!(Delay::new(3).is_err());
    let mut d = Delay::new(2048).unwrap();
    d.write_position = 77;
    d.counter = 99;
    d.feedback = 0.5;
    d.history[0] = [1., 2.];
    d.init();
    assert_eq!(d.write_position, 0);
    assert_eq!(d.counter, 0);
    assert_eq!(d.feedback, 0.5);
    assert!(d.history.iter().all(|v| *v == [0., 0.]));
    d.effect = 1.;
    let mut output = [[3., 4.]; 4];
    assert!(d.render_auxiliary(&mut output, &[]).is_err());
    assert_eq!(output, [[3., 4.]; 4]);
    assert_eq!(d.write_position, 0);
    d.render_channel(&mut []);
    d.render_auxiliary(&mut [], &[]).unwrap();
    assert_eq!(d.write_position, 0);
}
