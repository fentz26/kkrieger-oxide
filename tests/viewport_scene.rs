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
    doc.classes = [0xf0, 0xc4]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(2);
    for (index, op) in doc.operators.iter_mut().enumerate() {
        op.class = index;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    let mut p = vec![0; 25];
    p[0] = 3;
    p[1] = 3;
    p[2] = 0xff123456;
    p[9] = 100f32.to_bits();
    p[10] = 0.125f32.to_bits();
    p[13] = 1f32.to_bits();
    p[14] = 1f32.to_bits();
    p[16] = 90f32.to_bits();
    p[22] = 1f32.to_bits();
    p[23] = 1f32.to_bits();
    p[24] = 1;
    doc.operators[0].parameters = p;
    doc.operators[0].inputs = vec![1];
    doc.operators[1].parameters = vec![0; 11];
    doc.operators[1].parameters[9] = 1f32.to_bits();
    doc.operators[1].parameters[10] = 32f32.to_bits();
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
fn authored_viewport_executes_scene_and_selects_its_target() {
    let (doc, mut e) = setup();
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(9));
    assert_eq!(e.render_passes.len(), 1);
    let pass = &e.render_passes[0];
    assert_eq!(pass.viewport.window, [0, 0, 640, 360]);
    assert_eq!(pass.clear_flags, 3);
    assert_eq!(pass.clear_color, 0xff123456);
    assert_eq!(pass.paint_mode, 0);
    assert_eq!(pass.jobs.len(), 1);
}
#[test]
fn cached_viewport_skips_children_and_frame_resets_render_packets() {
    let (mut doc, mut e) = setup();
    e.frame(&doc, 0).unwrap();
    doc.classes[1].id = 0xffff;
    e.frame(&doc, 0).unwrap();
    assert!(e.render_passes.is_empty());
    assert!(e.jobs.is_empty());
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(9));
}
#[test]
fn cropped_stereo_passes_preserve_pool_master_and_camera_offsets() {
    let (mut doc, mut e) = setup();
    let p = &mut doc.operators[0].parameters;
    p[1] = 0x40 | 3;
    p[18] = 2f32.to_bits();
    p[19] = 4f32.to_bits();
    p[20] = 0.25f32.to_bits();
    p[21] = 0.25f32.to_bits();
    p[22] = 0.75f32.to_bits();
    p[23] = 0.75f32.to_bits();
    let pool = e.overlay.take();
    e = Execution::new(&doc);
    e.overlay = pool;
    e.intro_stereo = true;
    let saved = e.overlay.as_ref().unwrap().master;
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.render_passes.len(), 2);
    for (index, pass) in e.render_passes.iter().enumerate() {
        assert_eq!(pass.viewport.window, [160, 90, 480, 270]);
        assert_eq!(pass.camera.space[3][0], if index == 0 { -1. } else { 1. });
        assert_eq!(pass.camera.center_x, if index == 0 { 0.5 } else { -0.5 });
        assert_eq!(pass.allocate_stereo_target, index == 0);
        assert_eq!(pass.grab_stereo_target, index == 0);
        assert_eq!(pass.blend_stereo_target, index == 1);
    }
    assert_eq!(e.overlay.as_ref().unwrap().master, saved);
}
#[test]
fn empty_scene_uses_original_default_light_and_invalid_crop_keeps_full_view() {
    let (mut doc, mut e) = setup();
    doc.classes[1].id = 0xc1;
    doc.operators[1].parameters.clear();
    let p = &mut doc.operators[0].parameters;
    p[20] = 1f32.to_bits();
    p[21] = 1f32.to_bits();
    p[22] = 0;
    p[23] = 0;
    let pool = e.overlay.take();
    e = Execution::new(&doc);
    e.overlay = pool;
    e.frame(&doc, 0).unwrap();
    let pass = &e.render_passes[0];
    assert_eq!(pass.viewport.window, [0, 0, 640, 360]);
    let kkrieger_oxide::scene::exec::Job::Light {
        position,
        color,
        range,
        amplify,
        ..
    } = pass.jobs[0]
    else {
        panic!("missing fallback light")
    };
    assert_eq!(position, pass.camera.space[3]);
    assert_eq!(color, 0x78787878);
    assert_eq!(range, 32.);
    assert_eq!(amplify, 1.);
}
#[test]
fn unsupported_child_or_spline_does_not_leave_render_packets() {
    let (mut doc, mut e) = setup();
    doc.classes[1].id = 0xffff;
    assert!(e.frame(&doc, 0).is_err());
    assert!(e.render_passes.is_empty());
    let (mut doc, mut e) = setup();
    doc.operators[0].inputs.push(1);
    assert_eq!(
        e.frame(&doc, 0).unwrap_err().reason,
        "viewport camera spline execution has not been ported"
    );
    assert!(e.render_passes.is_empty());
}

#[test]
fn viewport_scene_precedes_its_parent_postprocessing_copy() {
    use kkrieger_oxide::scene::exec::RenderCommand;
    let (template, mut old) = setup();
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0xe2, 0xf0, 0xc4]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(3);
    for (index, op) in doc.operators.iter_mut().enumerate() {
        op.class = index;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].parameters = vec![4, 0xffffffff, 1f32.to_bits(), 1];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].parameters = template.operators[0].parameters.clone();
    doc.operators[1].inputs = vec![2];
    doc.operators[2].parameters = template.operators[1].parameters.clone();
    let mut e = Execution::new(&doc);
    e.overlay = old.overlay.take();
    e.frame(&doc, 0).unwrap();
    assert_eq!(
        e.render_commands,
        vec![RenderCommand::Scene(0), RenderCommand::Overlay(0)]
    );
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(12));
    assert_eq!(e.overlay_draws[0].texture, 109);
    assert_eq!(e.overlay.as_ref().unwrap().targets[9].references, 1);
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.render_commands, vec![RenderCommand::Overlay(0)]);
    assert!(e.render_passes.is_empty());
}
