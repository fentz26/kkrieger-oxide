use kkrieger_oxide::synth::allocation::Allocator;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
}
#[test]
fn original_voice_selection_and_tables_match_512_native_states() {
    let mut r = Reader(include_bytes!("fixtures/synth-allocation.bin"));
    assert_eq!(r.word(), 512);
    for case in 0..512 {
        let channel = r.word() as u8;
        let limit = r.word() as u8;
        let counter = r.word();
        let channels = std::array::from_fn(|_| r.word() as i32);
        let ages = std::array::from_fn(|_| r.word());
        let gates = std::array::from_fn(|_| r.word());
        let latest = std::array::from_fn(|_| r.word());
        let mut allocator = Allocator {
            channels,
            ages,
            latest,
            counter,
        };
        let selected = allocator.allocate(channel, limit, &gates).unwrap();
        assert_eq!(selected as u32, r.word(), "case {case} selected");
        assert_eq!(allocator.counter, r.word());
        for (field, value) in allocator
            .channels
            .map(|v| v as u32)
            .into_iter()
            .chain(allocator.ages)
            .chain(allocator.latest)
            .enumerate()
        {
            assert_eq!(value, r.word(), "case {case} table {field}");
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn initial_free_slots_and_invalid_allocations_preserve_native_order() {
    let mut allocator = Allocator::default();
    let gates = [1; 32];
    assert_eq!(allocator.allocate(3, 4, &gates).unwrap(), 0);
    assert_eq!(allocator.allocate(3, 4, &gates).unwrap(), 1);
    assert_eq!(allocator.latest[3], 1);
    let before = allocator.channels;
    assert!(allocator.allocate(16, 4, &gates).is_err());
    assert_eq!(allocator.channels, before);
    allocator.channels = [0; 32];
    allocator.counter = 0;
    allocator.ages = [1; 32];
    assert!(allocator.allocate(0, 1, &gates).is_err());
    assert_eq!(allocator.channels, [0; 32]);
}
