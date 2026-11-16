use kkrieger_oxide::scene::camera::Environment;
#[test]
fn original_projection_modes_half_pixel_offsets_and_zero_viewports() {
    struct Reader<'a>(&'a [u8]);
    impl Reader<'_> {
        fn word(&mut self) -> u32 {
            let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            n
        }
        fn float(&mut self) -> f32 {
            f32::from_bits(self.word())
        }
    }
    let mut r = Reader(include_bytes!("fixtures/projection.bin"));
    assert_eq!(r.word(), 512);
    for _ in 0..512 {
        let e = Environment {
            orthogonal: r.word() as i32,
            zoom_x: r.float(),
            zoom_y: r.float(),
            center_x: r.float(),
            center_y: r.float(),
            near_clip: r.float(),
            far_clip: r.float(),
            ..Default::default()
        };
        let viewport = [r.word() as i32, r.word() as i32];
        let matrix = e.projection(viewport).unwrap();
        for value in matrix.into_iter().flatten() {
            let expected = r.word();
            if value.is_nan() {
                assert!(f32::from_bits(expected).is_nan());
            } else {
                assert_eq!(value.to_bits(), expected);
            }
        }
    }
    assert!(r.0.is_empty());
    assert!(
        Environment {
            orthogonal: 2,
            ..Default::default()
        }
        .projection([640, 360])
        .is_err()
    );
}

#[test]
fn camera_material_bridge_preserves_lighting_and_fog() {
    let mut camera = Environment {
        model: [[2.; 4]; 4],
        light_position: [1., 2., 3., 1.],
        light_color: [0.5; 4],
        light_amplify: 2.,
        light_range: 32.,
        fog_start: 5.,
        fog_end: 90.,
        fog_color: 0xff123456,
        ..Default::default()
    };
    camera.space[3] = [3., 4., 5., 1.];
    let material = camera.material_environment([640, 360]).unwrap();
    assert_eq!(material.model, camera.model);
    assert_eq!(material.camera, camera.space);
    assert_eq!(material.world_to_camera[3], [-3., -4., -5., 1.]);
    assert_eq!(material.light_position, camera.light_position);
    assert_eq!(material.light_color, camera.light_color);
    assert_eq!(material.light_amplify, 2.);
    assert_eq!(material.light_range, 32.);
    assert_eq!(material.fog_start, 5.);
    assert_eq!(material.fog_end, 90.);
    assert_eq!(material.fog_color, 0xff123456);
}

#[test]
fn original_camera_to_uniform_matrices_match_native() {
    struct Reader<'a>(&'a [u8]);
    impl Reader<'_> {
        fn word(&mut self) -> u32 {
            let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            n
        }
        fn float(&mut self) -> f32 {
            f32::from_bits(self.word())
        }
        fn matrix(&mut self) -> [[f32; 4]; 4] {
            std::array::from_fn(|_| std::array::from_fn(|_| self.float()))
        }
    }
    let mut r = Reader(include_bytes!("fixtures/camera-bridge.bin"));
    assert_eq!(r.word(), 256);
    for _ in 0..256 {
        let mut e = Environment {
            orthogonal: r.word() as i32,
            zoom_x: r.float(),
            zoom_y: r.float(),
            center_x: r.float(),
            center_y: r.float(),
            near_clip: r.float(),
            far_clip: r.float(),
            ..Default::default()
        };
        let viewport = [r.word() as i32, r.word() as i32];
        e.space = r.matrix();
        let native_view = r.matrix();
        let native_view_projection = r.matrix();
        let material = e.material_environment(viewport).unwrap();
        for (actual, expected) in [material.world_to_camera, material.view_projection]
            .into_iter()
            .zip([native_view, native_view_projection])
        {
            for (a, b) in actual
                .into_iter()
                .flatten()
                .zip(expected.into_iter().flatten())
            {
                assert_eq!(a.to_bits(), b.to_bits());
            }
        }
    }
    assert!(r.0.is_empty());
}
