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
