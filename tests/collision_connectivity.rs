use kkrieger_oxide::collision::{Cell, connectivity};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        value
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
    fn list(&mut self) -> Vec<usize> {
        let count = self.word();
        (0..count).map(|_| self.word() as usize).collect()
    }
}

#[test]
fn original_heap_and_neighbor_order_match_for_128_graphs() {
    let mut r = Reader(include_bytes!("fixtures/collision-connectivity.bin"));
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let count = r.word() as usize;
        let cells: Vec<_> = (0..count)
            .map(|_| Cell {
                mode: r.word(),
                bounds: [r.vector(), r.vector()],
                vertices: [[0.; 4]; 8],
                planes: Vec::new(),
                only_for_shot: false,
                center: [0.; 4],
                interactions: Default::default(),
            })
            .collect();
        let graph = connectivity::connect(&cells);
        let sorted: Vec<_> = (0..count).map(|_| r.word() as usize).collect();
        assert_eq!(graph.sorted, sorted, "case {case}");
        for neighbors in &graph.neighbors {
            assert_eq!(neighbors.adds, r.list(), "case {case}");
            assert_eq!(neighbors.subs, r.list(), "case {case}");
            assert_eq!(neighbors.zones, r.list(), "case {case}");
        }
    }
    assert!(r.0.is_empty());
}
