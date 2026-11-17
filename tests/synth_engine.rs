use kkrieger_oxide::synth::engine::{Engine, Speech};
#[derive(Default)]
struct SpeechProbe {
    resets: usize,
    ticks: usize,
    on: usize,
    off: usize,
    renders: usize,
}
impl Speech for SpeechProbe {
    fn reset(&mut self, _: u32) -> Result<(), &'static str> {
        self.resets += 1;
        Ok(())
    }
    fn note_on(&mut self) -> Result<(), &'static str> {
        self.on += 1;
        Ok(())
    }
    fn note_off(&mut self) -> Result<(), &'static str> {
        self.off += 1;
        Ok(())
    }
    fn control(&mut self, _: u8, _: u8) -> Result<(), &'static str> {
        Ok(())
    }
    fn tick(&mut self) -> Result<(), &'static str> {
        self.ticks += 1;
        Ok(())
    }
    fn render(&mut self, _: &mut [[f32; 2]]) -> Result<(), &'static str> {
        self.renders += 1;
        Ok(())
    }
}
fn bank() -> Vec<u8> {
    let mut p = [0u8; 90];
    p[0] = 64;
    p[1] = 64;
    p[2] = 3;
    p[4] = 64;
    p[5] = 64;
    p[6] = 64;
    p[7] = 96;
    p[10] = 64;
    p[11] = 64;
    p[16] = 64;
    p[17] = 64;
    p[27] = 64;
    p[29] = 32;
    p[31] = 64;
    for n in [32, 38] {
        p[n + 1] = 64;
        p[n + 2] = 100;
        p[n + 3] = 64;
        p[n + 4] = 80;
        p[n + 5] = 100;
    }
    p[59] = 96;
    p[69] = 32;
    p[71] = 64;
    p[72] = 64;
    p[73] = 64;
    p[74] = 1;
    p[75] = 1;
    p[78] = 64;
    p[81] = 1;
    p[82] = 2;
    p[83] = 90;
    p[84] = 32;
    p[85] = 20;
    p[86] = 64;
    p[87] = 64;
    p[88] = 4;
    let mut bank = Vec::new();
    bank.extend(8u32.to_le_bytes());
    bank.extend(98u32.to_le_bytes());
    bank.extend(p);
    bank.extend(p);
    bank
}
fn globals() -> [u8; 23] {
    [
        64, 64, 32, 0, 64, 64, 1, 1, 0, 0, 64, 0, 127, 0, 0, 1, 2, 90, 32, 20, 64, 64, 0,
    ]
}
#[test]
fn rendered_audio_is_identical_across_arbitrary_output_fragments() {
    let bank = bank();
    let mut whole = Engine::new(&bank, 44100, 1234, SpeechProbe::default()).unwrap();
    let mut split = Engine::new(&bank, 44100, 1234, SpeechProbe::default()).unwrap();
    for engine in [&mut whole, &mut split] {
        engine.set_globals(&globals()).unwrap();
        engine.midi(&[0x90, 60, 100, 0xfd]).unwrap();
    }
    let mut expected = vec![[0.; 2]; 2048];
    whole.render(&mut expected).unwrap();
    let mut actual = vec![[0.; 2]; 2048];
    let mut offset = 0;
    for n in [1, 31, 96, 7, 511, 3, 1399] {
        split.render(&mut actual[offset..offset + n]).unwrap();
        offset += n;
    }
    assert_eq!(offset, actual.len());
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
    assert!(actual.iter().flatten().any(|v| v.abs() > 0.001));
    assert_eq!(whole.speech.ticks, 16);
    assert_eq!(split.speech.ticks, 16);
    assert_eq!(split.remaining(), 0);
}
#[test]
fn midi_changes_wait_for_the_next_cached_audio_frame_and_reset_restores_state() {
    let bank = bank();
    let mut whole = Engine::new(&bank, 44100, 1234, SpeechProbe::default()).unwrap();
    let mut split = Engine::new(&bank, 44100, 1234, SpeechProbe::default()).unwrap();
    for engine in [&mut whole, &mut split] {
        engine.set_globals(&globals()).unwrap();
        engine.midi(&[0x90, 60, 100, 0xfd]).unwrap();
    }
    let mut expected = [[0.; 2]; 128];
    whole.render(&mut expected).unwrap();
    let mut actual = [[0.; 2]; 128];
    split.render(&mut actual[..1]).unwrap();
    assert_eq!(split.remaining(), 127);
    split.midi(&[0xc0, 1, 0xfd]).unwrap();
    assert_eq!(split.allocator().channels, [-1; 32]);
    split.render(&mut actual[1..]).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(split.speech.ticks, 1);
    split.midi(&[0x9f, 60, 100, 0xfd]).unwrap();
    split.render(&mut [[0.; 2]; 128]).unwrap();
    assert_eq!(split.speech.on, 1);
    assert_eq!(split.speech.renders, 1);
    split.midi(&[0x8f, 60, 0, 0xfd]).unwrap();
    assert_eq!(split.speech.off, 1);
    split.midi(&[0xff, 0xfd]).unwrap();
    assert_eq!(split.speech.resets, 2);
    assert_eq!(split.allocator().channels, [-1; 32]);
    assert_eq!(split.remaining(), 0);
    assert!(split.voices().iter().all(|v| v.envelopes[0].state == 0));
    assert!(Engine::new(&bank, 192000, 0, SpeechProbe::default()).is_err());
}

#[test]
fn authored_music_and_vfx_packets_feed_the_composed_voice_and_effect_pipeline() {
    use kkrieger_oxide::{
        kx::Document,
        v2m::{Song, conversion::convert, sequence::Sequencer},
    };
    let document = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    assert_eq!(&document.samples[..4], b"VFX0");
    let n = u32::from_le_bytes(document.samples[4..8].try_into().unwrap()) as usize;
    for raw in [document.song, &document.samples[8..8 + n]] {
        let converted = convert(raw).unwrap();
        let song = Song::parse(&converted).unwrap();
        let mut sequence = Sequencer::new(&song, 44100).unwrap();
        let mut engine = Engine::new(song.patches, 44100, 1234, SpeechProbe::default()).unwrap();
        engine.set_globals(song.globals).unwrap();
        let mut had_voice = false;
        for _ in 0..48 {
            if !sequence.playing {
                break;
            }
            sequence.advance().unwrap();
            engine.midi(&sequence.midi).unwrap();
            had_voice |= engine
                .allocator()
                .channels
                .iter()
                .any(|&channel| channel >= 0);
            let mut audio = [[0.; 2]; 128];
            engine.render(&mut audio).unwrap();
            assert!(audio.iter().flatten().all(|v| v.is_finite()));
        }
        assert!(had_voice);
        assert!(engine.speech.ticks > 0);
    }
}
