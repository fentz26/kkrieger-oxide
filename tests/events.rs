use kkrieger_oxide::{
    kx::{Class, Document},
    mesh::identity,
    scene::{
        event::{Event, Scheduler},
        exec::Execution,
    },
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
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
}
#[test]
fn original_event_schedule_and_context_variables() {
    let mut r = Reader(include_bytes!("fixtures/events.bin"));
    assert_eq!(r.word(), 96);
    assert_eq!(r.word(), 8);
    for case in 0..96 {
        let mut schedule = Scheduler::default();
        let mut statics = Vec::new();
        for i in 0..8 {
            let event = Event {
                operator: Some(r.word() as usize),
                start: r.int(),
                end: r.int(),
                ..Event::default()
            };
            let id = schedule.insert(event, i >= 4);
            assert_eq!(id, i);
            if i < 4 {
                statics.push(id);
            }
        }
        for tick in 0..8 {
            let beat = r.int();
            let clear = r.word() != 0;
            let mut resets = schedule.begin(beat, clear);
            for &id in &statics {
                if schedule.add(id) && clear {
                    resets.push(id);
                }
            }
            assert_eq!(resets.len(), r.word() as usize, "{case}/{tick}");
            for op in 0..3 {
                let n = r.word() as usize;
                let expected: Vec<_> = (0..n).map(|_| u64::from(r.word())).collect();
                assert_eq!(schedule.active(op), expected);
            }
            let expired = schedule.finish();
            assert_eq!(expired.len(), r.word() as usize);
            let count = r.word();
            let remaining: Vec<_> = (0..count).map(|_| u64::from(r.word())).collect();
            assert_eq!(schedule.dynamic, remaining);
        }
    }
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let beat = r.int();
        let mut event = Event {
            start: r.int(),
            end: r.int(),
            interval: [r.float(), r.float()],
            velocity: r.float(),
            modulation: r.float(),
            select: r.int(),
            scale: r.vector(),
            rotate: r.vector(),
            translate: r.vector(),
            color: r.word(),
            ..Event::default()
        };
        let spline = r.word() != 0;
        let monster = r.word() != 0;
        let old_spline = r.word() != 0;
        for v in event.variables(beat) {
            for x in v {
                assert_eq!(x.to_bits(), r.word(), "context {case}");
            }
        }
        assert_eq!(monster, r.word() != 0);
        assert_eq!(spline || old_spline, r.word() != 0);
        assert_eq!(case, r.word());
        for i in 0..8 {
            assert_eq!(
                r.vector(),
                [i as f32, (i + 1) as f32, (i + 2) as f32, (i + 3) as f32]
            );
        }
        for _ in 0..3 {
            assert_eq!(r.word(), 1);
        }
        event.stop();
        assert_eq!(event.start, r.int());
        assert_eq!(event.end, r.int());
    }
    assert!(r.0.is_empty());
}
fn document() -> Document<'static> {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.classes = vec![
        Class {
            id: 6,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0x14,
            convention: 0,
            packing: &[],
        },
    ];
    doc.events.clear();
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![1f32.to_bits()];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![50, 8f32.to_bits(), 1f32.to_bits()];
    doc
}
#[test]
fn scene_events_isolate_instance_lists_restore_context_and_reset_expired_instances() {
    let doc = document();
    let mut execution = Execution::new(&doc);
    let mut matrix = identity();
    matrix[3] = [2., 3., 4., 1.];
    let first = execution.events.insert(
        Event {
            operator: Some(0),
            matrix,
            monster: Some(7),
            ..Event::default()
        },
        true,
    );
    let mut matrix2 = identity();
    matrix2[3] = [5., 6., 7., 1.];
    let second = execution.events.insert(
        Event {
            operator: Some(0),
            matrix: matrix2,
            ..Event::default()
        },
        true,
    );
    execution.vm.variables[0] = [9.; 4];
    execution.event_monster = Some(99);
    execution.event_spline = Some(0);
    execution.begin_events(32768, false);
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.events.events[&first].start, 32768);
    assert_eq!(execution.events.events[&first].end, 98304);
    // Last-added event executes first; both independent rising-edge slots fire.
    assert_eq!(execution.splash.position, [2., 3., 4., 1.]);
    assert!(execution.splash.enabled);
    assert_eq!(execution.vm.variables[0], [9.; 4]);
    assert_eq!(execution.event_monster, Some(99));
    assert_eq!(execution.event_spline, Some(0));
    assert_eq!(execution.current_event, None);
    execution.finish_events();
    execution.splash.enabled = false;
    execution.begin_events(40000, false);
    execution.frame(&doc, 0).unwrap();
    assert!(!execution.splash.enabled);
    execution.finish_events();
    execution.reset_event_instances(first);
    execution.begin_events(50000, false);
    execution.frame(&doc, 0).unwrap();
    assert!(execution.splash.enabled);
    execution.finish_events();
    execution.begin_events(100000, false);
    execution.frame(&doc, 0).unwrap();
    execution.finish_events();
    assert!(!execution.events.events.contains_key(&first));
    assert!(!execution.events.events.contains_key(&second));
}
#[test]
fn event_execution_restores_context_when_input_fails() {
    let mut doc = document();
    doc.classes[1].id = 0xffff;
    let mut execution = Execution::new(&doc);
    execution.vm.variables[0] = [7.; 4];
    execution.event_monster = Some(5);
    let id = execution.events.insert(
        Event {
            monster: Some(9),
            ..Event::default()
        },
        false,
    );
    assert!(execution.execute_event(&doc, 1, id, identity()).is_err());
    assert_eq!(execution.vm.variables[0], [7.; 4]);
    assert_eq!(execution.event_monster, Some(5));
    assert_eq!(execution.current_event, None);
}

#[test]
fn event_matrix_is_applied_before_parent_and_time_uses_only_newest_event() {
    let mut doc = document();
    doc.classes[0].id = 5;
    let mut execution = Execution::new(&doc);
    let first = execution.events.insert(
        Event {
            operator: Some(0),
            ..Event::default()
        },
        false,
    );
    let mut matrix = identity();
    matrix[3] = [2., 3., 4., 1.];
    let second = execution.events.insert(
        Event {
            operator: Some(0),
            matrix,
            ..Event::default()
        },
        false,
    );
    execution.begin_events(0, false);
    execution.events.add(first);
    execution.events.add(second);
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.splash.position, [2., 3., 4., 1.]);
    assert_eq!(execution.events.events[&second].start, 0);
    assert_eq!(execution.events.events[&second].end, 0);
    execution.reset_event_instances(second);
    let mut parent = identity();
    parent[0][0] = 2.;
    parent[1][1] = 3.;
    parent[2][2] = 4.;
    parent[3] = [10., 20., 30., 1.];
    execution.execute_event(&doc, 1, second, parent).unwrap();
    assert_eq!(execution.splash.position, [14., 29., 46., 1.]);
}
