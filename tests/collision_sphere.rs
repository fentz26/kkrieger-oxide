use kkrieger_oxide::collision::sphere;
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
    fn point(&mut self, actual: [f32; 3], case: usize, query: usize) {
        for a in actual {
            let b = f32::from_bits(self.word());
            assert_eq!(a.to_bits(), b.to_bits(), "{case}/{query}: {a} vs {b}");
        }
    }
}

#[test]
fn original_sphere_queries_and_response_preserve_all_output_writes() {
    let mut r = Reader(include_bytes!("fixtures/collision-sphere.bin"));
    assert_eq!(r.word(), 512);
    assert_eq!(r.word(), 8);
    let mut nan_responses = 0;
    for case in 0..512 {
        let count = r.word();
        let vertices: Vec<_> = (0..count).map(|_| r.vector()).collect();
        for query in 0..8 {
            let value = r.vector();
            let mut nearest = [11., 12., 13.];
            let hit = sphere::check_face(value, &vertices, &mut nearest);
            assert_eq!(hit, r.word() != 0, "face {case}/{query}");
            r.point(nearest, case, query);
            let mut response = value;
            sphere::collide_face(&mut response, &vertices);
            if response.iter().any(|x| x.is_nan()) {
                nan_responses += 1;
            }
            for (a, b) in response.into_iter().zip(r.vector()) {
                // The source oracle leaves invalid-operation NaN signs
                // host-dependent; payloads and non-NaN words remain exact.
                let bits = |value: f32| {
                    if value.is_nan() {
                        value.to_bits() & 0x7fff_ffff
                    } else {
                        value.to_bits()
                    }
                };
                assert_eq!(bits(a), bits(b), "response {case}/{query}: {a} vs {b}");
            }
            let edge = r.word() as usize;
            nearest = [11., 12., 13.];
            let hit = sphere::check_edge(
                value,
                vertices[edge],
                vertices[(edge + 1) % vertices.len()],
                &mut nearest,
            );
            assert_eq!(hit, r.word() != 0, "edge {case}/{query}");
            r.point(nearest, case, query);
        }
    }
    assert!(
        nan_responses > 0,
        "coincident source response cases must be covered"
    );
    assert!(r.0.is_empty());
}
