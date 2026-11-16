use kkrieger_oxide::{
    kx::{Class, Document},
    mesh::identity,
    scene::exec::{Execution, Job},
    visibility::{Portal, Rect},
};
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
    fn matrix(&mut self) -> [[f32; 4]; 4] {
        std::array::from_fn(|_| std::array::from_fn(|_| self.float()))
    }
    fn sectors(&mut self) -> Vec<(bool, Rect)> {
        (0..6)
            .map(|_| {
                let execute = self.word() != 0;
                let rect = Rect(std::array::from_fn(|_| self.float()));
                assert_eq!(self.word(), 0);
                (execute, rect)
            })
            .collect()
    }
}
fn check(e: &Execution, expected: &[(bool, Rect)], case: usize) {
    let actual: Vec<_> = e
        .jobs
        .iter()
        .filter_map(|job| {
            if let Job::Light { operator, clip, .. } = job {
                Some((*operator, *clip))
            } else {
                None
            }
        })
        .collect();
    let expected: Vec<_> = expected
        .iter()
        .enumerate()
        .rev()
        .filter(|(index, (execute, _))| !(index + case).is_multiple_of(3) && *execute)
        .map(|(index, (_, rect))| (index, *rect))
        .collect();
    assert_eq!(actual.len(), expected.len());
    for ((index, a), (expected_index, b)) in actual.into_iter().zip(expected) {
        assert_eq!(index, expected_index);
        for (a, b) in a.0.into_iter().zip(b.0) {
            assert!(
                a.to_bits() == b.to_bits() || (a - b).abs() < 2e-6 || a.is_nan() && b.is_nan(),
                "case {case}: {a} vs {b}"
            );
        }
    }
}
#[test]
fn native_visibility_traverses_cached_sectors_but_executes_only_draw_jobs() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![Class {
        id: 0xc4,
        convention: 0,
        packing: &[],
    }];
    doc.operators.truncate(6);
    for op in &mut doc.operators {
        op.class = 0;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![0; 11];
        op.parameters[9] = 1f32.to_bits();
        op.parameters[10] = 32f32.to_bits();
    }
    let mut r = Reader(include_bytes!("fixtures/cached-sectors.bin"));
    assert_eq!(r.word(), 64);
    for _ in 0..64 {
        let case = r.word() as usize;
        let projection = r.matrix();
        let observer = r.word();
        let observer = (observer != u32::MAX).then_some(observer as usize);
        assert_eq!(r.word(), 6);
        let count = r.word();
        let mut e = Execution::new(&doc);
        for i in 0..6 {
            if !(i + case).is_multiple_of(3) {
                e.jobs.push(Job::Sector {
                    operator: i,
                    input: i,
                    matrix: identity(),
                });
            }
        }
        for index in 0..count {
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
            for vertex in portal.cube {
                for value in vertex {
                    assert_eq!(r.word(), value.to_bits());
                }
            }
            e.jobs.push(Job::Portal {
                operator: 100 + index as usize,
                portal,
            });
        }
        let expected = r.sectors();
        e.process_sectors(&doc, observer, projection).unwrap();
        check(&e, &expected, case);
        let projection = r.matrix();
        let expected = r.sectors();
        e.jobs.retain(|job| !matches!(job, Job::Light { .. }));
        e.process_sectors(&doc, observer, projection).unwrap();
        check(&e, &expected, case);
    }
    assert!(r.0.is_empty());
}
