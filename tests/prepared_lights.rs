use kkrieger_oxide::{
    scene::{camera::Environment, exec::Job, light::Context},
    visibility::Rect,
};
fn job(position: [f32; 4], clip: Rect) -> Job {
    Job::Light {
        operator: 17,
        position,
        flags: 2,
        color: 0xff123456,
        amplify: 2.,
        range: 2.,
        clip,
    }
}
#[test]
fn original_light_preparation_applies_fade_and_builds_paint_frusta() {
    let context = Context::from_camera(&Environment::default(), [640, 360], true).unwrap();
    let light = context
        .prepare(&job([0., 0., 40., 1.], Rect::UNIT))
        .unwrap()
        .unwrap();
    assert_eq!(light.operator, 17);
    assert_eq!(light.amplify, 1.);
    assert_eq!(light.importance, 0.05);
    assert_eq!(light.color, 0xff123456);
    assert_eq!(light.shadow.count, 5);
    assert_eq!(light.zfail.count, 6);
    let paint = light.paint_light();
    assert_eq!(paint.flags, 2);
    assert_eq!(paint.position, [0., 0., 40.]);
    assert_eq!(paint.range, 2.);
    assert_eq!(paint.zfail_planes.len(), 6);
}
#[test]
fn original_sector_clip_participates_in_light_visibility() {
    let context = Context::from_camera(&Environment::default(), [640, 360], true).unwrap();
    let right = Rect([0.5, -1., 1., 1.]);
    assert!(
        context
            .prepare(&job([-6., 0., 10., 1.], Rect::UNIT))
            .unwrap()
            .is_some()
    );
    assert!(
        context
            .prepare(&job([-6., 0., 10., 1.], right))
            .unwrap()
            .is_none()
    );
    assert!(
        context
            .prepare(&job([6., 0., 10., 1.], right))
            .unwrap()
            .is_some()
    );
}
