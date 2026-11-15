use kkrieger_oxide::collision::{Cell, faces};
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
fn original_collision_face_clipping_and_list_order_match() {
    let mut r = Reader(include_bytes!("fixtures/collision-faces.bin"));
    assert_eq!(r.word(), 128);
    let mut maximum_planes = 0;
    for case in 0..128 {
        let count = r.word();
        let cells: Vec<_> = (0..count)
            .map(|_| {
                let mode = r.word();
                let cell = Cell::from_vertices(std::array::from_fn(|_| r.vector()), mode);
                maximum_planes = maximum_planes.max(cell.planes.len());
                cell
            })
            .collect();
        let count = r.word();
        let neighbors: Vec<_> = (0..count).map(|_| r.word() as usize).collect();
        let actual = faces::create(&cells, 0, &neighbors).unwrap();
        assert_eq!(actual.len(), r.word() as usize, "case {case}");
        for face in actual {
            assert_eq!(face.len(), r.word() as usize, "case {case}");
            for vertex in face {
                for (a, b) in vertex.into_iter().zip(r.vector()) {
                    assert_eq!(a.to_bits(), b.to_bits(), "case {case}: {a} vs {b}");
                }
            }
        }
    }
    assert_eq!(maximum_planes, 12);
    assert!(r.0.is_empty());
}
