use kkrieger_oxide::synth::speech::{Frame, Ronan};
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> &'a [u8] {
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        a
    }
    fn word(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn frame(&mut self, f: &Frame, case: usize, step: usize) {
        for (i, v) in f.values().into_iter().enumerate() {
            assert_eq!(
                v.to_bits(),
                self.word(),
                "case {case} step {step} frame {i}"
            );
        }
    }
    fn state(&mut self, r: &Ronan<'_>, case: usize, step: usize) {
        let s = &r.sequence;
        for (i, value) in [
            r.pitch.to_bits(),
            r.frame_rate as u32,
            s.frame_count as u32,
            s.position_in_syllable as u32,
            s.remaining as u32,
            s.syllable as u32,
            s.duration_factor as u32,
            s.inverse_duration.to_bits(),
            s.text_position as u32,
            s.phonemes[0] as u32,
            s.phonemes[1] as u32,
            u32::from(s.wait_on),
            u32::from(s.wait_off),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(value, self.word(), "case {case} step {step} sequencer {i}");
        }
        self.frame(&r.dsp.current, case, step);
        self.frame(&r.dsp.target, case, step);
        for (i, value) in r
            .dsp
            .filters
            .iter()
            .flat_map(|f| [f.p1.to_bits(), f.p2.to_bits()])
            .chain([
                r.dsp.last_input.to_bits(),
                r.dsp.seed,
                r.dsp.noise.to_bits(),
                r.dsp.highpass[0].to_bits(),
                r.dsp.highpass[1].to_bits(),
                r.dsp.equalizer.p1.to_bits(),
                r.dsp.equalizer.p2.to_bits(),
            ])
            .enumerate()
        {
            assert_eq!(value, self.word(), "case {case} step {step} DSP {i}");
        }
    }
}
#[test]
fn original_lyrics_all_syllables_controllers_sync_and_4096_render_ticks_match() {
    let mut r = Reader(include_bytes!("fixtures/synth-speech-sequence.bin"));
    let n = r.word() as usize;
    let texts: Vec<_> = (0..n)
        .map(|_| {
            let n = r.word() as usize;
            r.take(n)
        })
        .collect();
    let lyrics: [Option<&[u8]>; 64] = std::array::from_fn(|i| texts.get(i).copied());
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let sample_rate = r.word();
        let mut speech = Ronan::new(sample_rate).unwrap();
        speech.set_lyrics(&lyrics).unwrap();
        r.state(&speech, case, 999);
        for step in 0..64 {
            let action = r.word();
            let value = r.word() as u8;
            match action {
                1 => speech.note_on(),
                2 => speech.note_off(),
                3 => speech.set_control(4, value),
                4 => speech.set_control(5, value),
                5 => speech.initialize(sample_rate).unwrap(),
                _ => {}
            }
            speech.tick().unwrap();
            let n = r.word() as usize;
            let mut audio: Vec<_> = (0..n).map(|_| [r.float(), r.float()]).collect();
            speech.dsp.render(&mut audio);
            for (i, value) in audio.into_iter().flatten().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    r.word(),
                    "case {case} step {step} sample {i}"
                );
            }
            r.state(&speech, case, step);
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn real_speech_callbacks_run_inside_the_synth_engine() {
    use kkrieger_oxide::{
        kx::Document,
        synth::engine::Engine,
        v2m::{Song, conversion::convert},
    };
    let document = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let converted = convert(document.song).unwrap();
    let song = Song::parse(&converted).unwrap();
    let mut engine = Engine::new(song.patches, 44100, 1234, Ronan::default()).unwrap();
    engine.set_globals(song.globals).unwrap();
    let mut lyrics = [None; 64];
    lyrics[0] = Some(&b"!aa_"[..]);
    engine.speech.set_lyrics(&lyrics).unwrap();
    engine.render(&mut [[0.; 2]; 128]).unwrap();
    assert!(engine.speech.sequence.wait_on);
    engine.midi(&[0xbf, 4, 64, 0x9f, 60, 100, 0xfd]).unwrap();
    assert!(!engine.speech.sequence.wait_on);
    for _ in 0..64 {
        let mut audio = [[0.; 2]; 128];
        engine.render(&mut audio).unwrap();
        assert!(audio.iter().flatten().all(|v| v.is_finite()));
    }
    assert_ne!(engine.speech.dsp.seed, 0);
    assert!(engine.speech.sequence.wait_off);
    engine.midi(&[0x8f, 60, 0, 0xfd]).unwrap();
    assert!(!engine.speech.sequence.wait_off);
}

#[test]
fn lyric_bounds_and_the_original_zero_frame_rate_control_are_preserved() {
    let mut speech = Ronan::new(44100).unwrap();
    assert!(speech.set_lyrics(&[None; 63]).is_err());
    speech.set_control(4, 63);
    assert_eq!(speech.frame_rate, 0);
    speech.set_control(5, 128);
    let pitch = speech.pitch;
    speech.set_control(4, 8);
    assert_eq!(speech.pitch, pitch);
    assert_eq!(speech.frame_rate, 0);
    speech.initialize(48000).unwrap();
    assert_eq!(speech.pitch, 1.);
    assert_eq!(speech.frame_rate, 3);
}
