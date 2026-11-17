use kkrieger_oxide::synth::filter::{Filter, Parameters};
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
    fn check(&mut self, f: &Filter) {
        for word in [
            f.mode,
            f.frequency.to_bits(),
            f.resonance.to_bits(),
            f.low.to_bits(),
            f.band.to_bits(),
            f.step_bytes,
        ] {
            assert_eq!(word, self.word());
        }
    }
}
#[test]
fn original_x87_filter_coefficients_stride_aliasing_and_384_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-filter.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 3);
    for case in 0..128 {
        let p = Parameters {
            mode: r.float(),
            cutoff: r.float(),
            resonance: r.float(),
        };
        let scale = r.float();
        let step = r.word();
        let count = r.word() as usize;
        let alias = r.word() != 0;
        let length = count * step as usize / 4;
        let mut filter = Filter::default();
        filter.init();
        filter.step_bytes = step;
        filter.set(p, scale);
        r.check(&filter);
        for block in 0..3 {
            let source: Vec<_> = (0..length).map(|_| r.float()).collect();
            let mut output = if alias {
                source.clone()
            } else {
                vec![-777.; length]
            };
            if alias {
                filter.render_in_place(&mut output, count).unwrap();
            } else {
                filter.render(&mut output, &source, count).unwrap();
            }
            r.check(&filter);
            for (sample, value) in output.into_iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {sample}: {value}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn filter_init_preserves_coefficients_and_bad_buffers_do_not_mutate_state() {
    let mut f = Filter {
        mode: 2,
        frequency: 0.5,
        resonance: 0.25,
        low: 17.,
        band: 19.,
        step_bytes: 8,
    };
    f.init();
    assert_eq!(f.low, 0.);
    assert_eq!(f.band, 0.);
    assert_eq!(f.step_bytes, 4);
    assert_eq!(f.frequency, 0.5);
    assert_eq!(f.mode, 2);
    assert!(f.render(&mut [], &[1.], 1).is_err());
    assert_eq!(f.low, 0.);
    assert_eq!(f.band, 0.);
    f.step_bytes = 3;
    assert!(f.render_in_place(&mut [1.], 1).is_err());
    assert_eq!(f.low, 0.);
    assert_eq!(f.band, 0.);
    assert!(f.render_in_place(&mut [], 0).is_ok());
}
