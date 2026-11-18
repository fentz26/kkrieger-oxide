use kkrieger_oxide::{
    kx::{Class, Document, Layout},
    scene::exec::Execution,
};
#[test]
fn released_scene_wrapper_executes_inputs_without_authored_parameters() {
    let mut doc = Document::parse(kkrieger_oxide::embedded::data().unwrap()).unwrap();
    doc.layout = Layout::Beta2004;
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xe0,
            convention: 0x84000100,
            packing: &[],
        },
        Class {
            id: 0x0b,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.parameters.clear();
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![42, 19, 0, 0, 1];
    let mut e = Execution::new(&doc);
    e.switches[0] = 1;
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.switches[42], 19);
    doc.layout = Layout::Legacy3383;
    let mut e = Execution::new(&doc);
    assert!(e.frame(&doc, 0).is_err());
}
