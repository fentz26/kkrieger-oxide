use kkrieger_oxide::{
    kx::{Class, Document, Layout},
    scene::{
        exec::Execution,
        overlay::{Pool, Viewport},
    },
};
#[test]
fn released_viewport_executes_scene_then_allocates_and_caches_a_clear_pass() {
    let mut doc = Document::parse(kkrieger_oxide::embedded::data().unwrap()).unwrap();
    doc.layout = Layout::Beta2004;
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xf0,
            convention: 0x08000101,
            packing: &[],
        },
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
    doc.operators.truncate(4);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![3];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].inputs = vec![2];
    doc.operators[2].class = 2;
    doc.operators[2].parameters = vec![42, 19, 0, 0, 1];
    // A graph consumer gives the viewport one output reference.
    doc.operators[3].class = 0;
    doc.operators[3].inputs = vec![0];
    let mut e = Execution::new(&doc);
    e.switches[0] = 1;
    e.overlay = Some(Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 100, 800, 500],
        },
        std::array::from_fn(|i| 100 + i as i32),
    ));
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.switches[42], 19);
    assert_eq!(e.render_passes.len(), 1);
    let pass = &e.render_passes[0];
    assert_eq!(pass.viewport.window, [0, 0, 1024, 512]);
    assert_eq!(
        (pass.clear_flags, pass.clear_color, pass.paint_mode),
        (3, 0, 0)
    );
    let output = e.overlay.as_ref().unwrap().last_output.unwrap();
    assert_eq!(e.overlay.as_ref().unwrap().targets[output].owner, Some(0));
    e.switches[42] = 0;
    e.frame(&doc, 0).unwrap();
    assert_eq!(
        e.switches[42], 0,
        "cached path must not execute child scene"
    );
    assert!(e.render_passes.is_empty());
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(output));
}

#[test]
fn beta_viewport_full_texture_extent_and_fixed_screen_fallback_are_distinct() {
    let pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 100, 800, 500],
        },
        std::array::from_fn(|i| 100 + i as i32),
    );
    for (size, [width, height]) in kkrieger_oxide::scene::overlay::DIMENSIONS
        .into_iter()
        .enumerate()
    {
        let index = pool.get(size, 0).unwrap();
        assert_eq!(
            pool.beta_viewport(index).unwrap(),
            Viewport {
                screen: -1,
                render_target: 100 + index as i32,
                window: [0, 0, width, height]
            }
        );
    }
    assert_eq!(
        pool.beta_viewport(kkrieger_oxide::scene::overlay::SCREEN)
            .unwrap(),
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 128, 1024, 640]
        }
    );
    assert!(pool.beta_viewport(99).is_err());
}
