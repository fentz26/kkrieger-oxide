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
