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
