use kkrieger_oxide::scene::overlay::{Pool, SCREEN, Viewport};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn optional(&mut self) -> Option<usize> {
        let n = self.word();
        (n != u32::MAX).then_some(n as usize)
    }
}
fn pool() -> Pool {
    Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    )
}
#[test]
fn original_select_reference_adjustments_and_screen_reset_behavior() {
    let mut r = Reader(include_bytes!("fixtures/overlay-select.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 32);
    for _ in 0..64 {
        let mut pool = pool();
        for target in &mut pool.targets {
            target.references = r.int();
        }
        for _ in 0..32 {
            if r.word() != 0 {
                pool.reset();
            }
            let source = r.word() as usize;
            let count = r.int();
            pool.last_output = (source < 13).then_some(source);
            pool.select(count).unwrap();
            assert_eq!(r.word(), 1);
            assert_eq!(pool.last_output, r.optional());
            for target in &pool.targets {
                assert_eq!(target.references, r.int());
            }
            assert_eq!(r.word(), 0);
        }
    }
    assert!(r.0.is_empty());
    let mut pool = pool();
    pool.last_output = Some(SCREEN);
    pool.select(3).unwrap();
    pool.free(SCREEN).unwrap();
    assert_eq!(pool.targets[SCREEN].references, 2);
    pool.reset();
    assert_eq!(pool.targets[SCREEN].references, 2);
}
#[test]
fn scene_select_runs_inputs_and_adjusts_their_output_each_frame() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xea,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0xe6,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![3];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![0; 5];
    let mut e = Execution::new(&doc);
    let mut pool = pool();
    let source = pool.allocate(Some(1), 1, 2);
    pool.last_output = Some(SCREEN);
    e.overlay = Some(pool);
    for expected in [4, 6] {
        e.frame(&doc, 0).unwrap();
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.last_output, Some(source));
        assert_eq!(pool.targets[source].references, expected);
        assert!(e.overlay_draws.is_empty());
    }
}

#[test]
fn released_beta_select_has_no_parameter_or_reference_adjustment() {
    use kkrieger_oxide::{
        kx::{Class, Document, Layout},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.layout = Layout::Beta2004;
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xea,
            convention: 0x81000000,
            packing: &[],
        },
        Class {
            id: 0xe6,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![0; 5];
    let mut e = Execution::new(&doc);
    let mut pool = pool();
    let source = pool.allocate(Some(1), 1, 2);
    pool.last_output = Some(SCREEN);
    e.overlay = Some(pool);
    for _ in 0..2 {
        e.frame(&doc, 0).unwrap();
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.last_output, Some(source));
        assert_eq!(pool.targets[source].references, 2);
    }
}
