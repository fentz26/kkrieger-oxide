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
    doc.classes = [0x0d, 0xf0, 0xc1, 0x03]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(4);
    for (index, op) in doc.operators.iter_mut().enumerate() {
        op.class = index;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![];
    }
    doc.operators[0].inputs = vec![1];
    doc.operators[1].inputs = vec![2];
    let mut p = vec![0; 25];
    p[0] = 3;
    p[1] = 3;
    p[9] = 100f32.to_bits();
    p[10] = 0.125f32.to_bits();
    p[13] = 1f32.to_bits();
    p[14] = 1f32.to_bits();
    p[16] = 90f32.to_bits();
    p[22] = 1f32.to_bits();
    p[23] = 1f32.to_bits();
    p[24] = 1;
    doc.operators[1].parameters = p;
    let mut e = Execution::new(&doc);
    e.overlay = Some(Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    ));
    (doc, e)
}
#[test]
fn demo_root_executes_and_presents_authored_ipp_input() {
    let (doc, mut e) = setup();
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.render_passes.len(), 1);
    assert_eq!(e.overlay_draws.len(), 1);
    assert_eq!(e.overlay_draws[0].viewport.render_target, -1);
    assert_eq!(e.overlay_draws[0].texture, 109);
}

#[test]
fn each_ipp_branch_resets_cache_and_keeps_presentation_order() {
    use kkrieger_oxide::scene::exec::RenderCommand;
    let (mut doc, mut e) = setup();
    doc.operators[0].inputs = vec![1, 1];
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.render_passes.len(), 2);
    assert_eq!(e.overlay_draws.len(), 2);
    assert_eq!(
        e.render_commands,
        vec![
            RenderCommand::ResetOverlay,
            RenderCommand::Scene(0),
            RenderCommand::Overlay(0),
            RenderCommand::ResetOverlay,
            RenderCommand::Scene(1),
            RenderCommand::Overlay(1)
        ]
    );
    assert_eq!(e.overlay.as_ref().unwrap().targets[9].references, 1);
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(9));
}
#[test]
fn demo_handles_nested_demo_and_ipp_valued_aliases() {
    use kkrieger_oxide::scene::{PresentationKind, presentation_kind};
    for nested in [false, true] {
        let (mut doc, _) = setup();
        doc.operators[0].inputs = vec![3];
        doc.operators[3].inputs = vec![1];
        if nested {
            doc.operators[3].class = 0;
        }
        assert_eq!(
            presentation_kind(&doc, 3).unwrap(),
            if nested {
                PresentationKind::Demo
            } else {
                PresentationKind::Ipp
            }
        );
        let mut e = Execution::new(&doc);
        let (_, old) = setup();
        e.overlay = old.overlay;
        e.frame(&doc, 0).unwrap();
        assert_eq!(e.render_passes.len(), 1);
        assert_eq!(e.overlay_draws.len(), 1);
    }
}
#[test]
fn cache_alias_cycles_are_rejected_and_nonpresentation_input_is_not_executed() {
    use kkrieger_oxide::scene::presentation_kind;
    let (mut doc, mut e) = setup();
    doc.operators[0].inputs = vec![2];
    e.frame(&doc, 0).unwrap();
    assert!(e.render_commands.is_empty());
    doc.operators[3].inputs = vec![3];
    assert_eq!(
        presentation_kind(&doc, 3).unwrap_err().reason,
        "presentation cache alias cycle"
    );
}

#[test]
fn released_demo_cache_class_exposes_its_painter_scene() {
    use kkrieger_oxide::{
        game::root::painter_scene,
        kx::Layout,
        scene::{PresentationKind, presentation_kind},
    };
    let (mut doc, _) = setup();
    doc.classes[0].id = 0xe0;
    doc.layout = Layout::Beta2004;
    assert_eq!(presentation_kind(&doc, 0).unwrap(), PresentationKind::Demo);
    assert_eq!(painter_scene(&doc, 0).unwrap(), Some(2));
    doc.layout = Layout::Legacy3383;
    assert_eq!(presentation_kind(&doc, 0).unwrap(), PresentationKind::Other);
    assert_eq!(painter_scene(&doc, 0).unwrap(), None);
}
