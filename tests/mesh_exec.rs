use kkrieger_oxide::{
    kx::{Class, Document},
    scene::exec::{Execution, Job},
};
#[test]
fn direct_mesh_execution_visits_dependencies_without_submitting_geometry() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.classes = [0x81, 0x12, 0xc0]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0x1000_0000,
            packing: &[],
        })
        .collect();
    d.operators.truncate(3);
    for (i, op) in d.operators.iter_mut().enumerate() {
        op.class = i;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters.clear();
    }
    d.operators[0].inputs = vec![1];
    d.operators[1].parameters = vec![1];
    d.operators[1].links = vec![Some(0); 8];
    d.operators[2].parameters = [1f32, 1., 1., 0., 0., 0., 0., 0., 0.]
        .map(f32::to_bits)
        .to_vec();
    d.operators[2].inputs = vec![0];
    let mut e = Execution::new(&d);
    e.frame(&d, 0).unwrap();
    assert!(e.jobs.is_empty());
    assert_eq!(e.game.events.optics, [Some(0); 8]);
    e.game.events.optics = [None; 8];
    e.frame(&d, 2).unwrap();
    assert_eq!(e.jobs.len(), 1);
    assert!(matches!(e.jobs[0], Job::Paint { operator: 0, .. }));
    assert_eq!(e.game.events.optics, [None; 8]);
}
