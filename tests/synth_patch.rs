use kkrieger_oxide::synth::{
    patch::{Bank, Patch},
    voice::Voice,
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
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
#[test]
fn original_x87_voice_and_channel_modulation_match_512_patch_matrices() {
    let mut r = Reader(include_bytes!("fixtures/synth-patch.bin"));
    assert_eq!(r.word(), 512);
    for case in 0..512 {
        let base = r.take(88);
        let controls: [u8; 7] = r.take(7).try_into().unwrap();
        let mut voice = Voice {
            note: r.word() as i32,
            velocity: r.float(),
            ..Voice::default()
        };
        voice.envelopes[0].output = r.float();
        voice.envelopes[1].output = r.float();
        voice.lfos[0].output = r.float();
        voice.lfos[1].output = r.float();
        let n = r.word() as usize;
        let mods = r.take(n * 3);
        let mut data = base.to_vec();
        data.extend([1, n as u8]);
        data.extend(mods);
        let patch = Patch::parse(&data).unwrap();
        assert_eq!(patch.maximum_polyphony, 1);
        for (i, value) in patch
            .voice_values(&voice, &controls)
            .into_iter()
            .chain(patch.channel_values(&voice, &controls))
            .enumerate()
        {
            assert_eq!(value.to_bits(), r.word(), "case {case} parameter {i}");
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn truncated_patch_and_invalid_program_offsets_are_rejected() {
    let mut data = vec![0; 90];
    assert!(Patch::parse(&data).is_ok());
    for n in 0..90 {
        assert!(Patch::parse(&data[..n]).is_err());
    }
    data[89] = 1;
    assert!(Patch::parse(&data).is_err());
    data.extend([0, 64, 0]);
    assert!(Patch::parse(&data).is_ok());
    let mut bank = vec![0; 512];
    bank[..4].copy_from_slice(&512u32.to_le_bytes());
    bank.extend(data);
    let bank = Bank::new(&bank).unwrap();
    assert!(bank.patch(0).is_ok());
    assert!(bank.patch(128).is_err());
    assert!(Bank::new(&[]).is_err());
    let mut invalid = vec![255; 512];
    assert!(Bank::new(&invalid).is_err());
    invalid[..4].copy_from_slice(&511u32.to_le_bytes());
    assert!(Bank::new(&invalid).is_err());
}
#[test]
fn authored_music_uses_compact_program_offset_table() {
    let document =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let song = kkrieger_oxide::v2m::Song::parse(document.song).unwrap();
    let bank = Bank::new(song.patches).unwrap();
    assert_eq!(bank.program_count(), 13);
    assert!(bank.patch(13).is_err());
}
