use kkrieger_oxide::synth::midi::Decoder;
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
fn original_running_status_and_dispatch_match_1024_packets() {
    let mut r = Reader(include_bytes!("fixtures/synth-midi.bin"));
    assert_eq!(r.word(), 256);
    assert_eq!(r.word(), 4);
    for case in 0..256 {
        let mut decoder = Decoder {
            running_status: r.word() as u8,
        };
        for block in 0..4 {
            let n = r.word() as usize;
            let packet = r.take(n);
            let mut events = Vec::new();
            let stopped = decoder
                .process(packet, |message| {
                    events.push(message);
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                u32::from(decoder.running_status),
                r.word(),
                "case {case} block {block} status"
            );
            let count = r.word() as usize;
            assert_eq!(
                stopped,
                r.word() as usize,
                "case {case} block {block} cursor"
            );
            assert_eq!(events.len(), count);
            for (event, message) in events.into_iter().enumerate() {
                for value in [
                    u32::from(message.status),
                    u32::from(message.data[0]),
                    u32::from(message.data[1]),
                    u32::from(message.length),
                ] {
                    assert_eq!(value, r.word(), "case {case} block {block} event {event}");
                }
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn malformed_packets_are_bounded_and_callback_errors_propagate() {
    let mut d = Decoder::default();
    assert!(d.process(&[], |_| Ok(())).is_err());
    assert!(d.process(&[0x90, 60], |_| Ok(())).is_err());
    assert!(d.process(&[0xf0, 1, 0xfd], |_| Ok(())).is_err());
    assert_eq!(
        d.process(&[0x90, 60, 100, 0xfd], |_| Err("callback failed")),
        Err("callback failed")
    );
}

#[test]
fn complete_authored_music_and_sound_effect_packets_decode() {
    let mut r = Reader(include_bytes!("fixtures/v2m-sequence.bin"));
    assert_eq!(r.word(), 2);
    for expected_ticks in [1123, 236] {
        let n = r.word() as usize;
        r.take(n);
        r.take(48);
        let mut decoder = Decoder::default();
        let mut ticks = 0;
        let mut events = 0;
        while r.word() != 0 {
            r.take(48);
            let n = r.word() as usize;
            let packet = r.take(n);
            let position = decoder
                .process(packet, |_| {
                    events += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(position, n - 1);
            assert_eq!(packet[position], 0xfd);
            ticks += 1;
        }
        assert_eq!(ticks, expected_ticks);
        assert!(events > 100);
    }
    assert!(r.0.is_empty());
    let mut decoder = Decoder::default();
    assert_eq!(decoder.process(&[0xff, 1, 0xfd], |_| Ok(())).unwrap(), 1);
    assert_eq!(decoder.running_status, 0);
}
