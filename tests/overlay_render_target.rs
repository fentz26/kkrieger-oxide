use kkrieger_oxide::{
    kx::{Class, Document},
    scene::{
        exec::Execution,
        overlay::{Pool, Viewport},
    },
};
fn setup() -> (Document<'static>, Execution) {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0xeb, 0xe6, 0x26]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0x1000_0000,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(4);
    for (i, op) in doc.operators.iter_mut().enumerate() {
        op.animation = &[1];
        op.class = i.min(2);
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![];
    }
    doc.operators[0].inputs = vec![1];
    doc.operators[0].links = vec![Some(2)];
    doc.operators[1].parameters = vec![0; 5];
    let mut e = Execution::new(&doc);
    let mut pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [7, 11, 647, 371],
        },
        std::array::from_fn(|i| 100 + i as i32),
    );
    let output = pool.allocate(Some(1), 3, 2);
    pool.last_output = Some(output);
    e.overlay = Some(pool);
    (doc, e)
}
#[test]
fn linked_texture_receives_copy_then_restores_master_and_uv_scale() {
    let (doc, mut e) = setup();
    let saved = e.overlay.as_ref().unwrap().master;
    let viewport = Viewport {
        screen: -1,
        render_target: 900,
        window: [0, 0, 256, 128],
    };
    e.render_textures.insert(2, viewport);
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.overlay_draws.len(), 1);
    let draw = &e.overlay_draws[0];
    assert_eq!(draw.viewport, viewport);
    assert_eq!(draw.texture, 109);
    assert_eq!(draw.material, 27);
    assert_eq!(draw.color, 0xffffffff);
    assert_eq!(draw.vertices[2][4], 0.25);
    assert_eq!(draw.vertices[2][5], 0.25);
    let pool = e.overlay.as_ref().unwrap();
    assert_eq!(pool.master, saved);
    assert_eq!(pool.targets[9].scale_uv, [0.625, 0.703125]);
    assert_eq!(pool.targets[9].references, 2);
    assert_eq!(pool.last_output, Some(9));
}
#[test]
fn invalid_texture_skips_children_and_failed_child_restores_master() {
    let (mut doc, mut e) = setup();
    doc.classes[1].id = 0xffff;
    let saved = e.overlay.as_ref().unwrap().master;
    e.render_textures.insert(
        2,
        Viewport {
            screen: -1,
            render_target: -1,
            window: [0, 0, 256, 128],
        },
    );
    e.frame(&doc, 0).unwrap();
    assert!(e.overlay_draws.is_empty());
    assert_eq!(e.overlay.as_ref().unwrap().master, saved);
    e.render_textures.get_mut(&2).unwrap().render_target = 900;
    assert!(e.frame(&doc, 0).is_err());
    assert_eq!(e.overlay.as_ref().unwrap().master, saved);
    assert!(e.overlay_draws.is_empty());
}
#[test]
fn nested_target_wrappers_copy_to_inner_then_outer_and_keep_references() {
    let (mut doc, _) = setup();
    doc.operators[3].class = 0;
    doc.operators[0].inputs = vec![3];
    doc.operators[0].links = vec![Some(0)];
    doc.operators[3].inputs = vec![1];
    doc.operators[3].links = vec![Some(2)];
    let mut e = Execution::new(&doc);
    let mut pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    );
    pool.allocate(Some(1), 3, 2);
    e.overlay = Some(pool);
    e.render_textures.insert(
        0,
        Viewport {
            screen: -1,
            render_target: 800,
            window: [0, 0, 512, 256],
        },
    );
    e.render_textures.insert(
        2,
        Viewport {
            screen: -1,
            render_target: 900,
            window: [0, 0, 256, 128],
        },
    );
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.overlay_draws.len(), 2);
    assert_eq!(
        e.overlay_draws
            .iter()
            .map(|draw| draw.viewport.render_target)
            .collect::<Vec<_>>(),
        vec![900, 800]
    );
    assert_eq!(e.overlay.as_ref().unwrap().targets[9].references, 2);
    assert_eq!(e.overlay.as_ref().unwrap().master.render_target, -1);
}
