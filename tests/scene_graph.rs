use kkrieger_oxide::{
    kx::{Document, Layout},
    scene,
};
fn check(data: &[u8], layout: Layout) {
    let doc = Document::parse_layout(data, layout).unwrap();
    let roots: Vec<_> = doc
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(doc.classes[op.class].id, 0xc0..=0xc9 | 0xcb | 0xcd | 0xce))
        .map(|(i, _)| i)
        .collect();
    let scenes = scene::initialize_many(&doc, &roots).unwrap();
    assert_eq!(scenes.len(), roots.len());
    for (&id, node) in roots.iter().zip(&scenes) {
        let node = node.borrow();
        let op = &doc.operators[id];
        let class = doc.classes[op.class].id;
        if class == 0xcb {
            assert!(node.is_sector);
        }
        if class == 0xc2 {
            assert_eq!(node.count, op.parameters[9] as i32);
        }
        assert!(node.srt.iter().all(|v| v.is_finite()));
    }
}
#[test]
fn development_scene_initialization() {
    check(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
    );
}
#[test]
#[ignore = "requires independently extracted original beta"]
fn beta_scene_initialization() {
    check(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
    );
}

fn execute_supported(data: &[u8], layout: Layout) {
    use scene::exec::{Execution, Job};
    let doc = Document::parse_layout(data, layout).unwrap();
    let mut frames = 0;
    let mut paints = 0;
    let mut lights = 0;
    for (id, op) in doc.operators.iter().enumerate() {
        if !matches!(doc.classes[op.class].id, 0xc0..=0xc4 | 0xc6 | 0xcb) {
            continue;
        }
        let mut execution = Execution::new(&doc);
        execution.vm.variables[3] = [3., 4., 5., 6.];
        match execution.frame(&doc, id) {
            Ok(()) => {
                frames += 1;
                for job in &execution.jobs {
                    match job {
                        Job::Paint { matrix, .. } => {
                            paints += 1;
                            assert!(matrix.iter().flatten().all(|v| v.is_finite()));
                        }
                        Job::Light { position, .. } => {
                            lights += 1;
                            assert!(position.iter().all(|v| v.is_finite()));
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                assert!(execution.jobs.is_empty());
                assert_eq!(e.reason, "scene execution handler has not been ported");
            }
        }
        assert_eq!(execution.vm.variables[3], [3., 4., 5., 6.]);
    }
    assert!(frames > 0 && paints > 0 && lights > 0);
}
#[test]
fn development_supported_scene_jobs() {
    execute_supported(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
    );
}
#[test]
#[ignore = "requires independently extracted original beta"]
fn beta_supported_scene_jobs() {
    execute_supported(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
    );
}

fn deferred_sectors_and_portals(data: &[u8], layout: Layout) {
    use scene::exec::{Execution, Job};
    let doc = Document::parse_layout(data, layout).unwrap();
    let mut painted = 0;
    let mut pending = 0;
    for (id, op) in doc.operators.iter().enumerate() {
        if doc.classes[op.class].id != 0xcb {
            continue;
        }
        let mut execution = Execution::new(&doc);
        execution.frame(&doc, id).unwrap();
        assert!(
            execution
                .jobs
                .iter()
                .any(|j| matches!(j,Job::Sector {operator,..} if *operator==id))
        );
        match execution.process_sectors(&doc, None, kkrieger_oxide::mesh::identity()) {
            Ok(()) => {
                painted += 1;
                assert!(
                    execution
                        .jobs
                        .iter()
                        .any(|j| matches!(j, Job::Paint { .. }))
                );
            }
            Err(e) => {
                pending += 1;
                assert_eq!(e.reason, "scene execution handler has not been ported");
                assert!(execution.jobs.is_empty());
            }
        }
    }
    assert_eq!((painted, pending), (35, 12));
    let mut portals = 0;
    for (id, op) in doc.operators.iter().enumerate() {
        if doc.classes[op.class].id != 0xcd {
            continue;
        }
        let mut execution = Execution::new(&doc);
        if execution.frame(&doc, id).is_err() {
            assert!(execution.jobs.is_empty());
            continue;
        }
        let portal = execution
            .jobs
            .iter()
            .find_map(|job| match job {
                Job::Portal { operator, portal } if *operator == id => Some(portal),
                _ => None,
            })
            .unwrap();
        assert_eq!(portal.sectors, [op.links[0].unwrap(), op.links[1].unwrap()]);
        assert_eq!(portal.between, op.links[2]);
        portals += 1;
    }
    assert_eq!(portals, 35);
}
#[test]
fn development_deferred_sectors_and_portals() {
    deferred_sectors_and_portals(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
    );
}
#[test]
#[ignore = "requires independently extracted original beta"]
fn beta_deferred_sectors_and_portals() {
    deferred_sectors_and_portals(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
    );
}
