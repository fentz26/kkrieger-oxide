use kkrieger_oxide::{
    mesh::identity,
    scene::{
        culling::{Frustum, Plane},
        light::{Source, sphere_bounds, visibility},
    },
};
#[test]
fn original_light_bounds_keep_full_rect_when_camera_is_inside_sphere() {
    assert_eq!(
        sphere_bounds([0.; 3], 2., identity(), [1., 1.]),
        [-1., -1., 1., 1.]
    );
    let rect = sphere_bounds([0., 0., 10.], 1., identity(), [1., 1.]);
    assert!(rect[0] < -0.1 && rect[0] > -0.101);
    assert_eq!(rect[0], rect[1]);
    assert_eq!(rect[2], -rect[0]);
    assert_eq!(rect[3], rect[2]);
}
#[test]
fn original_light_visibility_uses_distance_fade_and_amplify_cutoff() {
    let frustum = Frustum {
        planes: [Plane::new([0., 0., 0., 1.]); 6],
        count: 5,
    };
    let mut light = Source {
        position: [0., 0., 40.],
        range: 2.,
        amplify: 1.,
    };
    let result = visibility(&light, [0.; 3], &frustum, true).unwrap();
    assert_eq!(result.importance, 0.05);
    assert_eq!(result.fade, 0.5);
    light.position[2] = 45.;
    assert!(visibility(&light, [0.; 3], &frustum, true).is_none());
    assert_eq!(
        visibility(&light, [0.; 3], &frustum, false).unwrap().fade,
        1.
    );
    light.position = [0.; 3];
    light.amplify = 1. / 255.;
    assert_eq!(
        visibility(&light, [0.; 3], &frustum, true)
            .unwrap()
            .importance,
        20.
    );
    light.amplify = 0.;
    assert!(visibility(&light, [0.; 3], &frustum, true).is_none());
    light.amplify = 1.;
    let outside = Frustum {
        planes: [Plane::new([0., 0., 0., -3.]); 6],
        count: 5,
    };
    assert!(visibility(&light, [0.; 3], &outside, false).is_none());
}
