use kkrieger_oxide::{
    music::{LOOP_FRAMES, Music},
    v2m::{Error, Song, conversion::convert, playback::Backend},
};
#[derive(Default)]
struct Constant {
    resets: usize,
}
impl Backend for Constant {
    fn reset(&mut self) -> Result<(), Error> {
        self.resets += 1;
        Ok(())
    }
    fn midi(&mut self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
    fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error> {
        output.fill([0.25, -2.0]);
        Ok(())
    }
}
#[test]
fn intro_handler_quantizes_pcm_and_reopens_after_entire_callback() {
    let doc =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let bytes = convert(doc.song).unwrap();
    let song = Song::parse(&bytes).unwrap();
    let mut music = Music::new(&song, Constant::default()).unwrap();
    assert_eq!(music.player.backend.resets, 2);
    music.timer = LOOP_FRAMES - 5001;
    let mut output = vec![[0.0; 2]; 5000];
    music.render(&mut output).unwrap();
    assert!(
        output
            .iter()
            .all(|&frame| frame == [8191.0 / 32768.0, -32767.0 / 32768.0])
    );
    assert_eq!(music.timer, LOOP_FRAMES - 1);
    assert_eq!(music.player.backend.resets, 2);
    music.render(&mut output[..2]).unwrap();
    assert_eq!(music.timer, 0); // overshoot discarded, not carried into next loop
    assert_eq!(music.player.backend.resets, 4); // Open Reset then Play Reset
}

#[test]
fn root_transition_silences_music_then_reopens_without_resetting_handler_timer() {
    let doc =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let bytes = convert(doc.song).unwrap();
    let song = Song::parse(&bytes).unwrap();
    let mut music = Music::new(&song, Constant::default()).unwrap();
    music.render(&mut [[0.0; 2]; 100]).unwrap();
    let sample = music.player.current_sample;
    music.pause();
    let mut output = [[1.0; 2]; 100];
    music.render(&mut output).unwrap();
    assert_eq!(output, [[0.0; 2]; 100]);
    assert_eq!(music.timer, 100);
    assert_eq!(music.player.current_sample, sample);
    music.restart().unwrap();
    assert_eq!(music.timer, 100);
    assert_eq!(music.player.backend.resets, 4);
    music.render(&mut output).unwrap();
    assert_eq!(music.timer, 200);
    assert!(output.iter().flatten().any(|v| *v != 0.0));
}

#[test]
#[ignore = "renders the full authored music cycle; run explicitly in release mode"]
fn complete_authored_music_cycle_renders_finite_audio_and_restarts() {
    use kkrieger_oxide::v2m::playback::Synth;
    struct Checked<'a> {
        synth: Synth<'a>,
        frames: usize,
        resets: usize,
        peak: f32,
    }
    impl Backend for Checked<'_> {
        fn reset(&mut self) -> Result<(), Error> {
            self.resets += 1;
            self.synth.reset()
        }
        fn midi(&mut self, packet: &[u8]) -> Result<(), Error> {
            self.synth.midi(packet)
        }
        fn render(&mut self, output: &mut [[f32; 2]]) -> Result<(), Error> {
            self.synth.render(output)?;
            assert!(
                output.iter().flatten().all(|v| v.is_finite()),
                "non-finite synth output after {} frames",
                self.frames
            );
            for value in output.iter().flatten() {
                self.peak = self.peak.max(value.abs());
            }
            self.frames += output.len();
            Ok(())
        }
    }
    let doc =
        kkrieger_oxide::kx::Document::parse(kkrieger_oxide::embedded::data().unwrap()).unwrap();
    let bytes = convert(doc.song).unwrap();
    let song = Song::parse(&bytes).unwrap();
    let backend = Checked {
        synth: Synth::new(&song, 44100, 12345).unwrap(),
        frames: 0,
        resets: 0,
        peak: 0.0,
    };
    let mut music = Music::new(&song, backend).unwrap();
    let mut output = [[0.0; 2]; 4096];
    let mut remaining = LOOP_FRAMES as usize;
    let mut callback = 0;
    let mut energy = [0.0f64; 5];
    while remaining != 0 {
        let count = [1, 127, 1024, 4096, 777][callback % 5].min(remaining);
        let window = ((LOOP_FRAMES as usize - remaining) / (30 * 44100)).min(4);
        music.render(&mut output[..count]).unwrap();
        assert!(output[..count].iter().flatten().all(|v| v.abs() <= 1.0));
        energy[window] += output[..count]
            .iter()
            .flatten()
            .map(|&v| f64::from(v) * f64::from(v))
            .sum::<f64>();
        remaining -= count;
        callback += 1;
    }
    assert_eq!(music.timer, 0);
    assert_eq!(music.player.backend.resets, 4);
    assert_eq!(music.player.backend.frames, LOOP_FRAMES as usize);
    assert!(
        energy.iter().all(|v| *v > 1.0),
        "silent music interval: {energy:?}"
    );
    assert!(music.player.backend.peak > 0.1);
    music.render(&mut output).unwrap();
    assert_eq!(music.timer, 4096);
    assert_eq!(music.player.backend.resets, 4);
    eprintln!(
        "full music cycle: peak={} energy={energy:?}",
        music.player.backend.peak
    );
}
