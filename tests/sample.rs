use kkrieger_oxide::scene::sample::{Output, Parameters, State};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn check(&mut self, n: f32) {
        assert_eq!(n.to_bits(), self.word());
    }
}
struct Capture<'a, 'b> {
    reader: &'a mut Reader<'b>,
    plays: u32,
    spatial: u32,
}
impl Output for Capture<'_, '_> {
    fn play(&mut self, sample: i32, volume: f32) -> i32 {
        self.plays += 1;
        assert_eq!(sample, self.reader.word() as i32);
        self.reader.check(volume);
        self.reader.word() as i32
    }
    fn spatial(
        &mut self,
        sample: i32,
        buffer: i32,
        position: [f32; 4],
        velocity: [f32; 4],
        near: f32,
        far: f32,
    ) {
        self.spatial += 1;
        assert_eq!(sample, self.reader.word() as i32);
        assert_eq!(buffer, self.reader.word() as i32);
        assert_eq!(position, self.reader.vector());
        for n in &velocity[..3] {
            self.reader.check(*n);
        }
        self.reader.check(near);
        self.reader.check(far);
    }
}
#[test]
fn original_sample_timing_switches_volume_and_spatial_updates() {
    let mut r = Reader(include_bytes!("fixtures/sample.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 64);
    for _ in 0..32 {
        let mut state = State::default();
        for step in 0..64 {
            let value = r.word() as i32;
            let switch = r.word() as u8;
            let threshold = r.float();
            let sample = r.word() as i32;
            let volume_db = r.float();
            let retrigger = r.float();
            let half_range = r.float();
            let delta = r.word() as i32;
            let position = r.vector();
            let plays = r.word();
            let spatial = r.word();
            let mut capture = Capture {
                reader: &mut r,
                plays: 0,
                spatial: 0,
            };
            state.advance(
                Parameters {
                    value,
                    threshold,
                    sample,
                    volume_db,
                    retrigger,
                    half_range,
                },
                switch,
                delta,
                position,
                Some(&mut capture),
            );
            assert_eq!(capture.plays, plays);
            assert_eq!(capture.spatial, spatial);
            assert_eq!(plays, r.word());
            assert_eq!(spatial, r.word());
            assert_eq!(u32::from(state.on), r.word());
            assert_eq!(state.retrigger, r.word() as i32);
            assert_eq!(u32::from(state.old_value), r.word());
            assert_eq!(state.old_position, r.vector());
            assert_eq!(state.buffer, r.word() as i32);
            assert_eq!(r.word(), step + 1);
        }
    }
    assert!(r.0.is_empty());
}
