use kkrieger_oxide::synth::speech::{Dsp, Frame, Rate, SYLLABLES, phonemes};
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
    fn byte(&mut self) -> u8 {
        let n = self.0[0];
        self.0 = &self.0[1..];
        n
    }
    fn frame(&mut self, f: &Frame, case: usize) {
        let actual = f.values();
        let expected: [u32; 25] = std::array::from_fn(|_| self.word());
        for (i, value) in actual.into_iter().enumerate() {
            assert_eq!(
                value.to_bits(),
                expected[i],
                "case {case} coefficient {i}: actual {:?}, expected {:?}",
                &actual[9..12],
                expected[9..12]
                    .iter()
                    .map(|&v| f32::from_bits(v))
                    .collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn original_phonemes_frames_and_384_complete_speech_dsp_blocks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-speech-dsp.bin"));
    let phonemes = phonemes();
    assert_eq!(r.word(), 69);
    for p in phonemes {
        for value in p.values {
            assert_eq!(value.to_bits(), r.word());
        }
    }
    assert_eq!(r.word(), 52);
    for s in SYLLABLES {
        for value in s.name {
            assert_eq!(value, r.byte());
        }
        for value in s.phonemes {
            assert_eq!(value as u8, r.byte());
        }
    }
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 3);
    for case in 0..128 {
        let rate = Rate::new(r.word()).unwrap();
        let n = r.word() as usize;
        let p1 = r.word() as usize;
        let p2 = r.word() as usize;
        let x = r.float();
        let pitch = r.float();
        let mut dsp = Dsp::new(rate);
        dsp.current = Frame::between(&phonemes[p1], &phonemes[p2], x, pitch, rate);
        dsp.target = Frame::between(&phonemes[p2], &phonemes[p1], 1. - x, pitch, rate);
        r.frame(&dsp.current, case);
        r.frame(&dsp.target, case);
        for block in 0..3 {
            let mut audio: Vec<_> = (0..n).map(|_| [r.float(), r.float()]).collect();
            dsp.render(&mut audio);
            r.frame(&dsp.current, case);
            let state: Vec<_> = dsp
                .filters
                .iter()
                .flat_map(|f| [f.p1.to_bits(), f.p2.to_bits()])
                .chain([
                    dsp.last_input.to_bits(),
                    dsp.seed,
                    dsp.noise.to_bits(),
                    dsp.highpass[0].to_bits(),
                    dsp.highpass[1].to_bits(),
                    dsp.equalizer.p1.to_bits(),
                    dsp.equalizer.p2.to_bits(),
                ])
                .collect();
            for (i, value) in state.into_iter().enumerate() {
                assert_eq!(value, r.word(), "case {case} block {block} state {i}");
            }
            for (i, value) in audio.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} block {block} sample {i}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn empty_speech_buffers_do_not_advance_interpolation_or_noise() {
    assert!(Rate::new(0).is_err());
    let rate = Rate::new(44100).unwrap();
    let p = phonemes();
    let mut dsp = Dsp::new(rate);
    dsp.current = Frame::between(&p[0], &p[18], 0.25, 1., rate);
    dsp.target = Frame::between(&p[18], &p[0], 0.75, 1., rate);
    dsp.seed = 123;
    dsp.filters[0].p1 = 2.;
    let current = dsp.current.values();
    let target = dsp.target.values();
    dsp.render(&mut []);
    assert_eq!(dsp.current.values(), current);
    assert_eq!(dsp.target.values(), target);
    assert_eq!(dsp.seed, 123);
    assert_eq!(dsp.filters[0].p1, 2.);
}
