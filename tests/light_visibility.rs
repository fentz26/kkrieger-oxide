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

#[test]
fn native_original_bounds_and_both_light_fade_configurations_match() {
    let mut bytes: &[u8] = include_bytes!("fixtures/light-visibility.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    fn real(bytes: &mut &[u8]) -> f32 {
        f32::from_bits(word(bytes))
    }
    assert_eq!(word(&mut bytes), 512);
    for case in 0..512 {
        let light = Source {
            position: std::array::from_fn(|_| real(&mut bytes)),
            range: real(&mut bytes),
            amplify: real(&mut bytes),
        };
        let view = std::array::from_fn(|_| std::array::from_fn(|_| real(&mut bytes)));
        let zoom = std::array::from_fn(|_| real(&mut bytes));
        let camera = std::array::from_fn(|_| real(&mut bytes));
        let projection = std::array::from_fn(|_| std::array::from_fn(|_| real(&mut bytes)));
        for value in sphere_bounds(light.position, light.range, view, zoom) {
            assert_eq!(
                value.to_bits(),
                word(&mut bytes),
                "case {case} projected bounds"
            );
        }
        let mut frustum = Frustum::from_view_projection(projection, [-1., -1., 1., 1.]);
        frustum.normalize();
        for kkrieger in [true, false] {
            let result = visibility(&light, camera, &frustum, kkrieger);
            assert_eq!(
                result.is_some(),
                word(&mut bytes) != 0,
                "case {case} KKRIEGER={kkrieger} visibility"
            );
            if let Some(result) = result {
                assert_eq!(
                    result.importance.to_bits(),
                    word(&mut bytes),
                    "case {case} importance"
                );
                assert_eq!(result.fade.to_bits(), word(&mut bytes), "case {case} fade");
            }
        }
    }
    assert!(bytes.is_empty());
}
