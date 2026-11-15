use kkrieger_oxide::{
    mesh::Matrix,
    visibility::{self, Portal},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn matrix(&mut self) -> Matrix {
        std::array::from_fn(|_| std::array::from_fn(|_| self.float()))
    }
}
#[test]
fn original_portal_projection_costs_and_visibility_match_cpp() {
    let mut r = Reader(include_bytes!("fixtures/visibility.bin"));
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let projection = r.matrix();
        let observer = r.word();
        let count = r.word() as usize;
        let portal_count = r.word();
        let mut portals = Vec::new();
        for _ in 0..portal_count {
            let sectors = [r.word() as usize, r.word() as usize];
            let between = r.word();
            let cost = r.word() as i32;
            let bounds = std::array::from_fn(|_| std::array::from_fn(|_| r.float()));
            let matrix = r.matrix();
            let portal = Portal::new(
                sectors,
                (between != u32::MAX).then_some(between as usize),
                bounds,
                matrix,
                cost,
            );
            for p in portal.cube {
                for v in p {
                    assert_eq!(v.to_bits(), r.word(), "case {case}");
                }
            }
            portals.push(portal);
        }
        let result = visibility::compute(
            count,
            &portals,
            (observer != u32::MAX).then_some(observer as usize),
            projection,
        )
        .unwrap();
        let previous: Vec<_> = result.iter().map(|s| s.bounds).collect();
        for sector in result {
            assert_eq!(r.word() != 0, sector.execute, "case {case}");
            for actual in sector.bounds.0 {
                let expected = r.float();
                assert!(
                    actual.to_bits() == expected.to_bits()
                        || actual.is_nan() && expected.is_nan()
                        || (actual - expected).abs() < 2e-6,
                    "case {case}: {actual} != {expected}"
                );
            }
            assert_eq!(r.word(), 0);
        }
        let projection = r.matrix();
        let result = visibility::compute_with_history(
            count,
            &portals,
            (observer != u32::MAX).then_some(observer as usize),
            projection,
            &previous,
        )
        .unwrap();
        for sector in result {
            assert_eq!(r.word() != 0, sector.execute, "repeated case {case}");
            for actual in sector.bounds.0 {
                let expected = r.float();
                assert!(
                    actual.to_bits() == expected.to_bits()
                        || actual.is_nan() && expected.is_nan()
                        || (actual - expected).abs() < 2e-6,
                    "repeated case {case}: {actual} != {expected}"
                );
            }
            assert_eq!(r.word(), 0);
        }
    }
    assert!(r.0.is_empty());
}
