use kkrieger_oxide::synth::{
    distortion::{Distortion, Parameters},
    filter::Filter,
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
    fn filter(&mut self, f: &Filter) {
        for n in [
            f.mode,
            f.frequency.to_bits(),
            f.resonance.to_bits(),
            f.low.to_bits(),
            f.band.to_bits(),
            f.step_bytes,
        ] {
            assert_eq!(n, self.word());
        }
    }
    fn distortion(&mut self, d: &Distortion) {
        for (i, n) in [
            d.mode,
            d.input_gain.to_bits(),
            d.output_gain.to_bits(),
            d.offset.to_bits(),
            d.crush_scale.to_bits(),
            d.crush_step as u32,
            d.crush_xor,
            d.counter,
            d.frequency,
            d.held_left.to_bits(),
            d.held_right.to_bits(),
            d.pre_coefficient.to_bits(),
            d.pre_left.to_bits(),
            d.pre_right.to_bits(),
            d.post_coefficient.to_bits(),
            d.post_left.to_bits(),
            d.post_right.to_bits(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(n, self.word(), "distortion field {i}");
        }
        self.filter(&d.left_filter);
        self.filter(&d.right_filter);
    }
}
#[test]
fn original_x87_distortion_set_and_384_mono_stereo_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-distortion.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 3);
    for case in 0..128 {
        let p = Parameters {
            mode: r.float(),
            input_gain: r.float(),
            parameter1: r.float(),
            parameter2: r.float(),
        };
        let stereo = r.word() != 0;
        let alias = r.word() != 0;
        let count = r.word() as usize;
        let length = count * if stereo { 2 } else { 1 };
        let mut distortion = Distortion::default();
        distortion.init();
        distortion.set(p, 1.).unwrap();
        r.distortion(&distortion);
        for block in 0..3 {
            let source: Vec<_> = (0..length).map(|_| r.float()).collect();
            let mut output = if alias {
                source.clone()
            } else {
                vec![-777.; length]
            };
            if alias {
                distortion.render_in_place(&mut output, stereo).unwrap();
            } else {
                distortion.render(&mut output, &source, stereo).unwrap();
            }
            r.distortion(&distortion);
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
fn invalid_distortion_modes_and_buffers_do_not_process_samples() {
    let mut d = Distortion::default();
    let p = Parameters {
        mode: 16.,
        input_gain: 32.,
        parameter1: 64.,
        parameter2: 64.,
    };
    assert!(d.set(p, 1.).is_err());
    assert_eq!(d.mode, 0);
    let mut buffer = [1., 2., 3.];
    assert!(d.render_in_place(&mut buffer, true).is_err());
    assert_eq!(buffer, [1., 2., 3.]);
    assert!(d.render(&mut buffer, &[], false).is_err());
    assert_eq!(buffer, [1., 2., 3.]);
    assert!(d.render_in_place(&mut [], true).is_ok());
}
