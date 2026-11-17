use kkrieger_oxide::v2m::{Song, sequence::Sequencer};
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
    fn snapshot(&mut self, s: &Sequencer<'_, '_>) {
        for value in [
            s.time,
            s.next_time,
            s.global.index,
            s.global.next_time,
            s.microseconds,
            s.numerator,
            s.denominator,
            s.ticks_per_quarter,
            s.bar,
            s.beat,
            s.tick,
            u32::from(s.playing),
        ] {
            assert_eq!(value, self.word(), "at song time {}", s.time);
        }
    }
}
#[test]
fn original_complete_song_and_vfx_midi_timelines_match_native() {
    let mut r = Reader(include_bytes!("fixtures/v2m-sequence.bin"));
    assert_eq!(r.word(), 2);
    for song_index in 0..2 {
        let n = r.word() as usize;
        let song = Song::parse(r.take(n)).unwrap();
        let mut sequence = Sequencer::new(&song, 44100).unwrap();
        r.snapshot(&sequence);
        let mut ticks = 0;
        let mut bytes = 0;
        while r.word() != 0 {
            sequence.advance().unwrap();
            r.snapshot(&sequence);
            let n = r.word() as usize;
            assert_eq!(
                sequence.midi,
                r.take(n),
                "song {song_index} time {}",
                sequence.time
            );
            ticks += 1;
            bytes += n;
        }
        assert!(!sequence.playing);
        assert!(ticks > 10);
        assert!(bytes > 100);
        sequence.advance().unwrap();
        assert!(sequence.midi.is_empty());
        eprintln!("song {song_index}: {ticks} original event ticks, {bytes} MIDI bytes");
    }
    assert!(r.0.is_empty());
}

#[test]
fn original_x86_sample_division_remainders_carries_and_faults_match() {
    use kkrieger_oxide::v2m::sequence::sample_delta;
    let mut r = Reader(include_bytes!("fixtures/v2m-sample-delta.bin"));
    assert_eq!(r.word(), 512);
    let mut successes = 0;
    let mut faults = 0;
    for _ in 0..512 {
        let next = r.word();
        let time = r.word();
        let tempo = r.word();
        let division = r.word();
        let initial = r.word();
        let success = r.word() != 0;
        let native_remainder = r.word();
        let native_samples = r.word();
        let mut remainder = initial;
        let result = sample_delta(next, time, tempo, division, &mut remainder);
        assert_eq!(result.is_ok(), success);
        assert_eq!(remainder, native_remainder);
        if success {
            successes += 1;
            assert_eq!(result.unwrap(), native_samples);
        } else {
            faults += 1;
        }
    }
    assert!(successes > 50 && faults > 50);
    assert!(r.0.is_empty());
    let mut remainder = u32::MAX;
    assert_eq!(sample_delta(1, 0, 1, 3, &mut remainder).unwrap(), 1);
    assert_eq!(remainder, 0);
}
