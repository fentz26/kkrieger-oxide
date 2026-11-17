use kkrieger_oxide::v2m::{
    Song,
    conversion::{convert, version_delta},
};
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
}
#[test]
fn original_converter_matches_authored_music_sound_effects_and_all_seven_versions() {
    let mut r = Reader(include_bytes!("fixtures/v2m-conversion.bin"));
    assert_eq!(r.word(), 6);
    let n = r.word() as usize;
    let parameter_versions: Vec<_> = (0..n).map(|_| r.word()).collect();
    r.take(n);
    let n = r.word() as usize;
    for _ in 0..n {
        r.word();
    }
    r.take(n);
    assert_eq!(r.word(), 9);
    for _ in 0..9 {
        let n = r.word() as usize;
        let data = r.take(n);
        let delta = r.word();
        assert_eq!(version_delta(data).unwrap(), delta as u8);
        let n = r.word() as usize;
        let expected = r.take(n);
        let output = convert(data).unwrap();
        assert_eq!(output, expected);
        let original = Song::parse(data).unwrap();
        if delta > 0 {
            let parameter_count = parameter_versions
                .iter()
                .filter(|&&v| v <= 6 - delta)
                .count();
            let bank_start = original.patches.as_ptr() as usize - data.as_ptr() as usize;
            let first_patch =
                u32::from_le_bytes(original.patches[..4].try_into().unwrap()) as usize;
            if original.patches[first_patch + parameter_count] != 0 {
                let mut invalid = data.to_vec();
                invalid[bank_start + first_patch + parameter_count + 3] = 255;
                assert!(convert(&invalid).is_err());
            }
        }

        assert_eq!(version_delta(&output).unwrap(), 0);
        let mut padded = output.clone();
        padded.extend([0; 4]);
        assert_eq!(convert(&output).unwrap(), padded);
        let song = Song::parse(&output).unwrap();
        assert_eq!(song.global_events.data, original.global_events.data);
        for (new, old) in song.channels.iter().zip(original.channels.iter()) {
            for (a, b) in [new.notes, new.programs, new.pitch]
                .into_iter()
                .chain(new.controls)
                .zip(
                    [old.notes, old.programs, old.pitch]
                        .into_iter()
                        .chain(old.controls),
                )
            {
                assert_eq!(a.count, b.count);
                assert_eq!(a.data, b.data);
            }
        }
        let bank = kkrieger_oxide::synth::patch::Bank::new(song.patches).unwrap();
        for program in 0..bank.program_count() {
            let patch = bank.patch(program as u8).unwrap();
            let values = patch.voice_values(&Default::default(), &[0; 7]);
            kkrieger_oxide::synth::voice::Parameters::from_values(&values).unwrap();
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn conversion_rejects_empty_and_truncated_input() {
    assert!(convert(&[]).is_err());
    assert!(convert(&[0; 88]).is_err());
}
