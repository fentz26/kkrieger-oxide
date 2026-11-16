use kkrieger_oxide::{
    kx::{Class, Document},
    mesh::identity,
    scene::exec::{Execution, Job},
    visibility::{Portal, Rect},
};
#[test]
fn current_camera_drives_portal_clip_rectangles_in_deferred_sector_execution() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0xc4]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.class = 0;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![0; 11];
        op.parameters[9] = 1f32.to_bits();
        op.parameters[10] = 32f32.to_bits();
    }
    for offset in [0., 0.5] {
        let mut e = Execution::new(&doc);
        e.current_camera.space[3][0] = offset;
        let portal = Portal::new(
            [10, 20],
            None,
            [[-0.5, -0.5, 2.], [0.5, 0.5, 3.]],
            identity(),
            1,
        );
        e.jobs = vec![
            Job::Sector {
                operator: 10,
                input: 0,
                matrix: identity(),
            },
            Job::Sector {
                operator: 20,
                input: 1,
                matrix: identity(),
            },
            Job::Portal {
                operator: 30,
                portal,
            },
        ];
        e.process_camera_sectors(&doc, Some(10), [640, 360])
            .unwrap();
        let clips: Vec<_> = e
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
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].0, 1);
        assert_eq!(clips[1], (0, Rect::UNIT));
        let expected = [
            (-0.5 - offset - 1. / 640.) / 2.,
            (-0.5 + 1. / 360.) / 2.,
            (0.5 - offset - 1. / 640.) / if offset == 0. { 2. } else { 3. },
            (0.5 + 1. / 360.) / 2.,
        ];
        for (actual, expected) in clips[0].1.0.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
        }
    }
}
