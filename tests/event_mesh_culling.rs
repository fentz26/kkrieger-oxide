use kkrieger_oxide::{
    kx::{Class, Document},
    mesh::identity,
    scene::{
        event::Event,
        exec::{Execution, Job},
    },
};
#[test]
fn original_event_distance_culls_only_meshes_and_uses_game_camera() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.operators.truncate(3);
    d.classes = [0x81, 0x64, 0xc1]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    for (i, op) in d.operators.iter_mut().enumerate() {
        op.class = i;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters.clear();
    }
    d.operators[2].inputs = vec![0, 1];
    let mut e = Execution::new(&d);
    e.game_camera.space[3] = [1., 2., 3., 1.];
    e.current_camera.space[3] = [100., 100., 100., 1.];
    for (position, cull_distance, expected_mesh) in [
        ([4., 6., 3., 1.], 5, true),
        ([4., 6., 4., 1.], 5, false),
        ([40., 60., 30., 1.], 0, true),
        ([40., 60., 30., 1.], -1, true),
    ] {
        let mut matrix = identity();
        matrix[3] = position;
        let id = e.events.insert(
            Event {
                matrix,
                cull_distance,
                ..Event::default()
            },
            false,
        );
        e.jobs.clear();
        e.execute_event(&d, 2, id, identity()).unwrap();
        assert_eq!(
            e.jobs
                .iter()
                .any(|j| matches!(j, Job::Paint { operator: 0, .. })),
            expected_mesh
        );
        assert!(
            e.jobs
                .iter()
                .any(|j| matches!(j, Job::Paint { operator: 1, .. }))
        );
        assert!(e.current_event.is_none());
    }
}
