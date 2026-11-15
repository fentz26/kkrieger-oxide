use kkrieger_oxide::collision::Cell;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
}
#[test]
fn original_collision_cells_include_warped_and_degenerate_faces() {
    let mut r = Reader(include_bytes!("fixtures/collision.bin"));
    assert_eq!(r.word(), 256);
    for case in 0..256 {
        let mode = r.word();
        let vertices = std::array::from_fn(|_| r.vector());
        let cell = Cell::from_vertices(vertices, mode);
        assert_eq!(cell.mode, r.word());
        assert_eq!(u32::from(cell.only_for_shot), r.word());
        assert_eq!(cell.planes.len(), r.word() as usize, "case {case}");
        for actual in
            cell.planes
                .iter()
                .copied()
                .chain([cell.center, cell.bounds[0], cell.bounds[1]])
        {
            let expected = r.vector();
            for (a, b) in actual.into_iter().zip(expected) {
                assert_eq!(a.to_bits(), b.to_bits(), "case {case}: {a} vs {b}");
            }
        }
    }
    assert!(r.0.is_empty());
}
