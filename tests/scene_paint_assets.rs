use kkrieger_oxide::{kx::Document, scene::paint::prepare_scene_assets};
#[test]
fn scene_dependency_preparation_includes_meshes_and_effect_materials() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let material = d
        .operators
        .iter()
        .find(|o| d.classes[o.class].id == 0xd0)
        .unwrap()
        .parameters
        .clone();
    d.operators.truncate(4);
    let classes =
        [0x81, 0x61, 0xd0, 0xc1].map(|id| d.classes.iter().position(|c| c.id == id).unwrap());
    for (i, op) in d.operators.iter_mut().enumerate() {
        op.class = classes[i];
        op.inputs.clear();
        op.links.clear();
        op.animation = &[1];
        op.parameters.clear();
    }
    d.operators[0].parameters = vec![
        1,
        1,
        1,
        0,
        1f32.to_bits(),
        1f32.to_bits(),
        1f32.to_bits(),
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    d.operators[1].links = vec![Some(2)];
    d.operators[2].parameters = material;
    d.operators[3].inputs = vec![0, 1];
    let assets = prepare_scene_assets(&d, 3, true).unwrap();
    assert_eq!(assets.meshes.keys().copied().collect::<Vec<_>>(), [0]);
    assert!(assets.materials.contains_key(&2));
    assert!(prepare_scene_assets(&d, 9, true).is_err());
}
#[test]
#[ignore = "prepares authored scene geometry"]
fn authored_menu_and_gameplay_roots_prepare_scene_paint_dependencies() {
    use kkrieger_oxide::{
        mesh_graph,
        scene::{
            exec::{Execution, Job},
            overlay::{Pool, Viewport},
        },
    };
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    for root_index in [1, 2] {
        let root = d.roots[root_index].unwrap();
        let assets = prepare_scene_assets(&d, root, true).unwrap();
        if root_index == 1 {
            let mut e = Execution::new_player(&d);
            e.overlay = Some(Pool::new(
                Viewport {
                    screen: 0,
                    render_target: -1,
                    window: [0, 0, 640, 360],
                },
                std::array::from_fn(|i| 100 + i as i32),
            ));
            e.frame(&d, root).unwrap();
            let queued=e.render_passes.iter().flat_map(|p|&p.jobs).filter(|j|matches!(j,Job::Paint {operator,..} if mesh_graph::implemented(d.classes[d.operators[*operator].class].id))).count();
            assert_eq!(queued, 0);
            assert!(assets.meshes.is_empty());
        } else {
            assert!(!assets.meshes.is_empty());
            assert!(!assets.materials.is_empty());
        }
        eprintln!(
            "root {root_index} paint meshes={} materials={}",
            assets.meshes.len(),
            assets.materials.len()
        );
    }
}
