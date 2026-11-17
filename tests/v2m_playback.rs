use kkrieger_oxide::v2m::{
    Error, Song,
    playback::{Backend, Playback},
};
#[derive(Default)]
struct Synth {
    phase: u32,
    hash: u32,
    trace: Vec<u32>,
}
impl Backend for Synth {
    fn reset(&mut self) -> Result<(), Error> {
        self.phase = 0;
        self.hash = 0;
        self.trace.extend([0, 1, 2]);
        Ok(())
    }
    fn midi(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.trace.extend([3, bytes.len() as u32]);
        for &b in bytes {
            self.trace.push(u32::from(b));
            self.hash = self.hash.wrapping_mul(33) ^ u32::from(b);
        }
        Ok(())
    }
    fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error> {
        self.trace.extend([4, output.len() as u32]);
        for frame in output {
            self.phase = self.phase.wrapping_add(1);
            *frame = [
                ((self.phase ^ self.hash) & 65535) as f32 / 65536. - 0.5,
                ((self.phase.wrapping_mul(7) ^ self.hash) >> 8 & 65535) as f32 / 65536. - 0.5,
            ];
        }
        Ok(())
    }
}
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
    fn snapshot(&mut self, p: &mut Playback<'_, '_, Synth>, case: usize, step: usize) {
        for (i, value) in [
            u32::from(p.valid),
            p.state as u32,
            p.sequence.time,
            p.sequence.next_time,
            p.sequence.microseconds,
            p.sequence.bar,
            p.sequence.beat,
            p.sequence.tick,
            p.current_sample,
            p.sample_delta,
            p.remainder,
            p.time_offset as u32,
            p.backend.phase,
            p.backend.hash,
            p.fade_value.to_bits(),
            p.fade_delta.to_bits(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(value, self.word(), "case {case} step {step} field {i}");
        }
        assert_eq!(p.backend.trace.len(), self.word() as usize);
        for (i, value) in p.backend.trace.drain(..).enumerate() {
            assert_eq!(value, self.word(), "case {case} step {step} synth call {i}");
        }
    }
}
#[test]
fn original_play_seek_stop_fade_render_and_eof_match_153_operations() {
    let mut r = Reader(include_bytes!("fixtures/v2m-playback.bin"));
    assert_eq!(r.word(), 9);
    for case in 0..9 {
        let n = r.word() as usize;
        let song = Song::parse(r.take(n)).unwrap();
        let rate = r.word();
        let tpc = r.word();
        let mut player = Playback::new(&song, rate, tpc, Synth::default()).unwrap();
        r.snapshot(&mut player, case, 999);
        let count = r.word();
        for step in 0..count {
            let op = r.word();
            let value = r.word();
            match op {
                0 => {
                    let mut output = vec![[0.375; 2]; value as usize * 2];
                    let playing = player.render(&mut output, value as usize).unwrap();
                    assert_eq!(u32::from(playing), r.word());
                    for (i, v) in output.into_iter().flatten().enumerate() {
                        assert_eq!(v.to_bits(), r.word(), "case {case} step {step} sample {i}");
                    }
                }
                1 => player.play(value).unwrap(),
                2 => player.stop(value).unwrap(),
                _ => player.close().unwrap(),
            }
            r.snapshot(&mut player, case, step as usize);
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn fades_require_a_bounded_tail_before_any_audio_or_state_changes() {
    use kkrieger_oxide::v2m::playback::{State, samples_for_time};
    let document =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let song = Song::parse(document.song).unwrap();
    let mut player = Playback::new(&song, 44100, 1000, Synth::default()).unwrap();
    player.play(0).unwrap();
    player.stop(10).unwrap();
    let mut output = [[0.375; 2]; 16];
    let calls = player.backend.trace.clone();
    assert!(player.render(&mut output, 16).is_err());
    assert_eq!(output, [[0.375; 2]; 16]);
    assert_eq!(player.backend.trace, calls);
    assert_eq!(player.current_sample, 0);
    assert_eq!(player.state, State::Playing);
    let mut timing = Playback::new(&song, 0, 1000, Synth::default()).unwrap();
    timing.play(0).unwrap();
    assert!(timing.backend.trace.is_empty());
    assert!(!timing.render(&mut output, 16).unwrap());
    assert_eq!(output, [[0.; 2]; 16]);
    assert!(samples_for_time(1, 44100, 0).is_err());
    assert!(samples_for_time(i32::MAX as u32, 44100, 1).is_err());
    assert_eq!(samples_for_time(1000, 44100, 1000).unwrap(), 44100);
    assert_eq!(
        samples_for_time(u32::MAX, 44100, 1000).unwrap(),
        (-44i32) as u32
    );
}

#[test]
fn real_synth_playback_preserves_audio_across_event_and_output_fragments() {
    use kkrieger_oxide::{
        kx::Document,
        v2m::{conversion::convert, playback::Synth as Audio},
    };
    let document = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let converted = convert(document.song).unwrap();
    let song = Song::parse(&converted).unwrap();
    let mut whole =
        Playback::new(&song, 44100, 1000, Audio::new(&song, 44100, 1234).unwrap()).unwrap();
    let mut split =
        Playback::new(&song, 44100, 1000, Audio::new(&song, 44100, 1234).unwrap()).unwrap();
    whole.play(22000).unwrap();
    split.play(22000).unwrap();
    let mut expected = vec![[0.; 2]; 8192];
    whole.render(&mut expected, 8192).unwrap();
    let mut actual = vec![[0.; 2]; 8192];
    let mut offset = 0;
    for n in [1, 31, 96, 7, 511, 3, 1399, 6144] {
        split.render(&mut actual[offset..offset + n], n).unwrap();
        offset += n;
    }
    assert_eq!(
        actual
            .iter()
            .flatten()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .flatten()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    assert!(actual.iter().flatten().all(|v| v.is_finite()));
    let peak = actual
        .iter()
        .flatten()
        .fold(0.0f32, |peak, v| peak.max(v.abs()));
    assert!(peak > 0.001, "authored playback peak {peak}");
    assert_eq!(whole.current_sample, split.current_sample);
    assert_eq!(whole.sample_delta, split.sample_delta);
}
