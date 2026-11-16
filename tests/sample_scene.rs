use kkrieger_oxide::{
    kx::{Class, Document},
    scene::exec::Execution,
};
#[test]
fn authored_sample_operator_executes_its_child() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0x0c, 0x12]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0x1000_0000,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(2);
    for (i, op) in doc.operators.iter_mut().enumerate() {
        op.class = i;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].parameters = vec![0, 1, 1f32.to_bits(), 4, 6f32.to_bits(), 0, 0];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].parameters = vec![1];
    doc.operators[1].links = vec![Some(0); 8];
    let mut e = Execution::new(&doc);
    e.switches[0] = 1;
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.game.events.optics, [Some(0); 8]);
}

#[test]
fn sample_instance_reuses_playback_state_until_view_reset() {
    use kkrieger_oxide::scene::sample::Output;
    use std::{cell::RefCell, rc::Rc};
    struct Audio(Rc<RefCell<Vec<(i32, f32)>>>);
    impl Output for Audio {
        fn play(&mut self, sample: i32, volume: f32) -> i32 {
            self.0.borrow_mut().push((sample, volume));
            7
        }
        fn spatial(
            &mut self,
            _sample: i32,
            _buffer: i32,
            _position: [f32; 4],
            _velocity: [f32; 4],
            _near: f32,
            _far: f32,
        ) {
        }
    }
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![Class {
        id: 0x0c,
        convention: 0,
        packing: &[],
    }];
    doc.operators.truncate(1);
    let op = &mut doc.operators[0];
    op.class = 0;
    op.animation = &[1];
    op.inputs.clear();
    op.links.clear();
    op.parameters = vec![0, 1, 1f32.to_bits(), 4, 0, 0, 0];
    let calls = Rc::new(RefCell::new(vec![]));
    let mut e = Execution::new(&doc);
    e.audio = Some(Box::new(Audio(calls.clone())));
    e.switches[0] = 1;
    e.frame(&doc, 0).unwrap();
    e.frame(&doc, 0).unwrap();
    assert_eq!(&*calls.borrow(), &[(4, 1.)]);
    e.switches[0] = 0;
    e.frame(&doc, 0).unwrap();
    e.switches[0] = 1;
    e.frame(&doc, 0).unwrap();
    assert_eq!(calls.borrow().len(), 2);
    e.reset_view();
    e.frame(&doc, 0).unwrap();
    assert_eq!(calls.borrow().len(), 3);
}
