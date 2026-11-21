use kkrieger_oxide::{
    kx::Layout,
    scene::{camera::Environment, overlay::fx_quad_for_layout},
};
#[test]
fn released_fx_quad_and_projection_match_native_device_boundary() {
    let mut words = include_bytes!("fixtures/beta-overlay-quad.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v));
    assert_eq!(words.next(), Some(64));
    for case in 0..64 {
        let viewport = [words.next().unwrap() as i32, words.next().unwrap() as i32];
        let provided = [words.next().unwrap() != 0, words.next().unwrap() != 0];
        let uv: [[f32; 4]; 2] =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let e = Environment {
            orthogonal: 2,
            released_projection: true,
            ..Default::default()
        };
        assert_eq!(e.orthogonal as u32, words.next().unwrap());
        let vertices = fx_quad_for_layout(
            Layout::Beta2004,
            viewport,
            None,
            provided[0].then_some(uv[0]),
            None,
            provided[1].then_some(uv[1]),
        );
        for (index, value) in vertices.into_iter().flatten().enumerate() {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case{case} vertexword{index}"
            );
        }
        let projection = e.projection(viewport).unwrap();
        for (index, value) in projection.into_iter().flatten().enumerate() {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case{case} projection{index}"
            );
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn beta_pool_emits_pixel_space_draws_and_development_keeps_normalized_draws() {
    use kkrieger_oxide::scene::overlay::{COUNT, Pool, SCREEN, Viewport};
    let viewport = Viewport {
        screen: 0,
        render_target: -1,
        window: [0, 0, 640, 480],
    };
    for layout in [Layout::Beta2004, Layout::Legacy3383] {
        let pool = Pool::new_for_layout(
            viewport,
            std::array::from_fn::<_, COUNT, _>(|i| 100 + i as i32),
            layout,
        );
        let draw = pool.copy(0, SCREEN, 0, 0xffffffff, 1.).unwrap().unwrap();
        let released = layout == Layout::Beta2004;
        assert_eq!(draw.environment.released_projection, released);
        assert_eq!(draw.environment.orthogonal, if released { 2 } else { 1 });
        assert_eq!(
            &draw.vertices[0][..3],
            if released {
                &[-0.5, -0.5, 1.]
            } else {
                &[-1., 1., 1.]
            }
        );
        assert_eq!(
            &draw.vertices[2][..3],
            if released {
                &[639.5, 479.5, 1.]
            } else {
                &[1., -1., 1.]
            }
        );
        let projection = draw.environment.projection([640, 480]).unwrap();
        assert_eq!(projection[2][2], if released { 1. } else { 1. / 4096. });
    }
}
