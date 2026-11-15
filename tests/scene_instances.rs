use kkrieger_oxide::scene::instance::{Memory, Scope};

#[test]
fn sequential_instances_and_nested_scene_scopes_replay_independently() {
    let mut memory = Memory::default();
    memory.enter(Scope::Default);
    memory.delay(12).unwrap().time = 0.25;
    // A repeated operator in the same traversal consumes a distinct instance.
    memory.delay(12).unwrap().time = 0.5;
    let saved = memory.enter(Scope::Scene(3));
    memory.delay(12).unwrap().time = 0.75;
    let nested = memory.enter(Scope::Scene(4));
    memory.delay(12).unwrap().time = 1.;
    memory.restore(nested);
    memory.delay(13).unwrap().time = 1.25;
    memory.restore(saved);
    memory.delay(14).unwrap().time = 1.5;

    for _ in 0..3 {
        memory.enter(Scope::Default);
        assert_eq!(memory.delay(12).unwrap().time, 0.25);
        assert_eq!(memory.delay(12).unwrap().time, 0.5);
        let saved = memory.enter(Scope::Scene(3));
        assert_eq!(memory.delay(12).unwrap().time, 0.75);
        let nested = memory.enter(Scope::Scene(4));
        assert_eq!(memory.delay(12).unwrap().time, 1.);
        memory.restore(nested);
        assert_eq!(memory.delay(13).unwrap().time, 1.25);
        memory.restore(saved);
        assert_eq!(memory.delay(14).unwrap().time, 1.5);
        assert_eq!(
            memory.delay(15).unwrap_err().reason,
            "instance list exhausted during replay"
        );
    }
    memory.enter(Scope::Default);
    assert_eq!(
        memory.delay(15).unwrap_err().reason,
        "instance operator differs during replay"
    );
}

#[test]
fn delay_and_if_execute_children_and_retain_per_branch_state() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::{Execution, Job},
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.classes = [0x09, 0x08, 0xc4]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(5);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![0, 1];
    doc.operators[0].inputs = vec![1, 2];
    for (i, op) in doc.operators.iter_mut().enumerate().take(3).skip(1) {
        op.class = 1;
        op.parameters = vec![0x10, 1, 0, 0, 1f32.to_bits(), 0, 1f32.to_bits()];
        op.inputs = vec![i + 2];
    }
    for op in &mut doc.operators[3..5] {
        op.class = 2;
        op.parameters = vec![0; 11];
        op.parameters[10] = 1f32.to_bits();
    }
    let mut execution = Execution::new(&doc);
    execution.switches[1] = 1;
    execution.current_time = 250;
    execution.switches[0] = 1;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.vm.variables[1], [0.25; 4]);
    assert!(matches!(
        execution.jobs.as_slice(),
        [Job::Light { operator: 3, .. }]
    ));
    execution.switches[0] = 0;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.vm.variables[1], [0.25; 4]);
    assert!(matches!(
        execution.jobs.as_slice(),
        [Job::Light { operator: 4, .. }]
    ));
    execution.switches[0] = 1;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.vm.variables[1], [0.5; 4]);
    execution.switches[0] = 0;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.vm.variables[1], [0.5; 4]);
}

#[test]
fn physics_handler_reuses_cells_and_consumes_motion() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::{Execution, Job},
    };
    use std::rc::Rc;
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.classes = [0xce, 0xc4]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.links.clear();
        op.inputs.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![
        2 | 0x10,
        0,
        0,
        0,
        2f32.to_bits(),
        4f32.to_bits(),
        6f32.to_bits(),
        0,
        0,
        0,
        0,
        0,
    ];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![0; 11];
    let mut execution = Execution::new(&doc);
    execution.event_monster = Some(17);
    execution.frame(&doc, 0).unwrap();
    assert!(matches!(
        execution.jobs.as_slice(),
        [Job::Light { operator: 1, .. }]
    ));
    let cell = Rc::clone(&execution.dynamic_cells[0]);
    assert_eq!(cell.borrow().monster, Some(17));
    assert_eq!(cell.borrow().cell.mode, 0x21);
    execution.physics(0.5);
    assert_eq!(cell.borrow().cell.mode, 1);
    cell.borrow_mut().matrix1[3][0] = 5.;
    cell.borrow_mut().blocked = 1;
    execution.finish_collision_tick();
    assert_eq!(cell.borrow().matrix0[3][0], 0.);
    cell.borrow_mut().blocked = 0;
    execution.finish_collision_tick();
    assert_eq!(cell.borrow().matrix0[3][0], 5.);
    execution.event_monster = None;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.dynamic_cells.len(), 1);
    assert!(Rc::ptr_eq(&cell, &execution.dynamic_cells[0]));
    assert_eq!(cell.borrow().monster, None);
    assert_eq!(cell.borrow().matrix1[3][0], 0.);
    execution.physics(0.5);
    assert_eq!(cell.borrow().cell.center[0], 2.5);
}
