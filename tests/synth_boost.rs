use kkrieger_oxide::synth::{boost::Boost, rate::Rate};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        value
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn state(&mut self, b: &Boost, case: usize, block: usize) {
        for (field, value) in [
            b.enabled,
            b.a1.to_bits(),
            b.a2.to_bits(),
            b.b0.to_bits(),
            b.b1.to_bits(),
            b.b2.to_bits(),
            b.x1[0].to_bits(),
            b.x1[1].to_bits(),
            b.x2[0].to_bits(),
            b.x2[1].to_bits(),
            b.y1[0].to_bits(),
            b.y1[1].to_bits(),
            b.y2[0].to_bits(),
            b.y2[1].to_bits(),
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
fn original_x87_bass_coefficients_and_512_persistent_stereo_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-boost.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 4);
    for case in 0..128 {
        let rate = Rate::new(r.word()).unwrap();
        let amount = r.float();
        let count = r.word() as usize;
        let mut boost = Boost::default();
        boost.init();
        boost.set(amount, &rate);
        r.state(&boost, case, 99);
        for block in 0..4 {
            boost.set(r.float(), &rate);
            let mut audio: Vec<_> = (0..count).map(|_| [r.float(), r.float()]).collect();
            boost.render(&mut audio);
            r.state(&boost, case, block);
            for (sample, value) in audio.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {sample}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn initializer_and_disabled_filter_preserve_all_history() {
    let mut boost = Boost {
        x1: [1., 2.],
        y2: [3., 4.],
        b0: 5.,
        ..Boost::default()
    };
    boost.init();
    assert_eq!(boost.x1, [1., 2.]);
    assert_eq!(boost.y2, [3., 4.]);
    assert_eq!(boost.b0, 5.);
    let rate = Rate::new(44100).unwrap();
    boost.set(0.5, &rate);
    assert_eq!(boost.enabled, 0);
    let mut audio = [[-0., 1.], [2., 3.]];
    let original = audio;
    boost.render(&mut audio);
    assert_eq!(
        audio.map(|v| v.map(f32::to_bits)),
        original.map(|v| v.map(f32::to_bits))
    );
    assert_eq!(boost.x1, [1., 2.]);
    assert_eq!(boost.y2, [3., 4.]);
    assert_eq!(boost.b0, 5.);
    boost.render(&mut []);
}
