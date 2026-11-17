use kkrieger_oxide::{kx::Document, v2m::Song};
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> &'a [u8] {
        let (d, rest) = self.0.split_at(n);
        self.0 = rest;
        d
    }
    fn word(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }
}
#[test]
fn authored_song_and_sound_effect_base_tables_match_original_loader() {
    let mut r = Reader(include_bytes!("fixtures/v2m-base.bin"));
    assert_eq!(r.word(), 2);
    for _ in 0..2 {
        let n = r.word() as usize;
        let data = r.take(n);
        let song = Song::parse(data).unwrap();
        assert_eq!(r.word(), 1);
        for value in [
            song.time_division,
            song.scaled_time_division,
            song.maximum_time,
            song.global_events.count,
        ] {
            assert_eq!(value, r.word());
        }
        let offset = |s: &[u8]| (s.as_ptr() as usize - data.as_ptr() as usize) as u32;
        assert_eq!(offset(song.global_events.data), r.word());
        for c in song.channels {
            assert_eq!(c.notes.count, r.word());
            if c.notes.count != 0 {
                assert_eq!(offset(c.notes.data), r.word());
                for stream in [c.programs, c.pitch].into_iter().chain(c.controls) {
                    assert_eq!(stream.count, r.word());
                    assert_eq!(offset(stream.data), r.word());
                }
            }
        }
        assert_eq!(offset(song.globals), r.word());
        assert_eq!(offset(song.patches), r.word());
        assert_eq!(song.speech_data.map_or(u32::MAX, offset), r.word());
        for text in song.speech {
            let n = r.word();
            if n == u32::MAX {
                assert!(text.is_none());
            } else {
                assert_eq!(text.unwrap(), r.take(n as usize));
            }
        }
        assert!(song.consumed <= data.len());
    }
    assert!(r.0.is_empty());
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    assert_eq!(Song::parse(doc.song).unwrap().time_division, 96);
}
#[test]
fn malformed_v2m_counts_sizes_and_speech_tables_are_rejected() {
    let empty = vec![0u8; 88];
    assert!(Song::parse(&empty).is_ok());
    for n in 0..88 {
        assert!(Song::parse(&empty[..n]).is_err(), "prefix {n}");
    }
    let mut bad = empty.clone();
    bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Song::parse(&bad).is_err());
    let mut bad = empty.clone();
    bad[76..80].copy_from_slice(&16385u32.to_le_bytes());
    assert!(Song::parse(&bad).is_err());
    let mut bad = empty.clone();
    bad[80..84].copy_from_slice(&1048577u32.to_le_bytes());
    assert!(Song::parse(&bad).is_err());
    let mut bad = empty.clone();
    bad[84..88].copy_from_slice(&4u32.to_le_bytes());
    bad.extend_from_slice(&257u32.to_le_bytes());
    assert!(Song::parse(&bad).is_err());
    let mut ignored = empty;
    ignored[84..88].copy_from_slice(&8192u32.to_le_bytes());
    assert_eq!(Song::parse(&ignored).unwrap().consumed, 88);
}
