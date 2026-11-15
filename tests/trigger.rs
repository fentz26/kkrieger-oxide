use kkrieger_oxide::{
    mesh::identity,
    scene::trigger::{Parameters, State},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector<const N: usize>(&mut self) -> [f32; N] {
        std::array::from_fn(|_| self.float())
    }
}
#[test]
fn original_trigger_state_and_created_events() {
    let mut r = Reader(include_bytes!("fixtures/trigger.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 16);
    let mut total = 0;
    for case in 0..128 {
        let valid = r.word() != 0;
        let flags = r.word();
        let mut state = State::default();
        let mut calls = 0;
        for tick in 0..16 {
            let beat = r.int();
            let p = Parameters {
                operator: 4,
                flags,
                trigger: r.float(),
                threshold: r.float(),
                delay: r.float(),
                duration: r.float(),
                velocity: r.float(),
                modulation: r.float(),
                select: r.int(),
                scale: r.vector(),
                rotate: r.vector(),
                translate: r.vector(),
            };
            let mut parent = identity();
            parent[3] = r.vector();
            let event = if valid {
                calls += 1;
                state.advance(beat, p, parent)
            } else {
                None
            };
            assert_eq!(state.active, r.word() != 0, "{case}/{tick}");
            assert_eq!(state.time, r.int(), "{case}/{tick}");
            assert_eq!(calls, r.word());
            assert_eq!(tick + 1, r.word());
            assert_eq!(u32::from(event.is_some()), r.word());
            if let Some(event) = event {
                total += 1;
                assert_eq!(event.operator, Some(r.word() as usize));
                assert_eq!(event.start, r.int());
                assert_eq!(event.end, r.int());
                assert_eq!(r.word(), 1);
                assert_eq!(event.velocity.to_bits(), r.word());
                assert_eq!(event.modulation.to_bits(), r.word());
                assert_eq!(event.select, r.int());
                for v in [event.scale, event.rotate, event.translate] {
                    for x in v {
                        assert_eq!(x.to_bits(), r.word());
                    }
                }
                assert_eq!(event.color, r.word());
                assert_eq!(event.spline.is_some(), r.word() != 0);
                assert_eq!(event.interval, r.vector());
                assert_eq!(event.matrix, identity());
            }
        }
    }
    assert!(total > 100);
    assert!(r.0.is_empty());
}
#[test]
fn scene_trigger_registers_for_next_frame_and_keeps_event_instances_separate() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [7, 6, 0x14, 0xc3]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(4);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].links = vec![Some(1)];
    doc.operators[0].inputs = vec![1];
    doc.operators[0].parameters = vec![
        1f32.to_bits(),
        0,
        0.25f32.to_bits(),
        0.5f32.to_bits(),
        6,
        0.75f32.to_bits(),
        0.5f32.to_bits(),
        3,
        1f32.to_bits(),
        1f32.to_bits(),
        1f32.to_bits(),
        0,
        0,
        0,
        0,
        0,
        0,
        0x12345678,
    ];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![0.5f32.to_bits()];
    doc.operators[1].inputs = vec![2];
    doc.operators[2].class = 2;
    doc.operators[2].parameters = vec![10, 2f32.to_bits(), 1f32.to_bits()];
    doc.operators[3].class = 3;
    doc.operators[3].parameters = [1f32, 1., 1., 0., 0., 0., 2., 3., 4.]
        .map(f32::to_bits)
        .to_vec();
    doc.operators[3].inputs = vec![0];
    let mut execution = Execution::new(&doc);
    for (beat, count, fired) in [
        (0, 1, false),
        (8192, 1, true),
        (16384, 2, false),
        (24576, 2, true),
    ] {
        execution.splash.enabled = false;
        execution.begin_events(beat, false);
        execution.frame(&doc, 3).unwrap();
        assert_eq!(execution.events.dynamic.len(), count);
        assert_eq!(execution.splash.enabled, fired);
        for event in execution.events.events.values() {
            let event = event.borrow();
            assert_eq!(event.translate, [2., 3., 4., 0.]);
            assert_eq!(event.color, 0);
            assert_eq!(event.spline, None);
        }
        execution.finish_events();
    }
}
