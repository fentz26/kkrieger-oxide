use kkrieger_oxide::{
    gpu_target::attachment_dimensions,
    scene::overlay::{COUNT, DIMENSIONS, Pool, Viewport},
};
#[test]
fn cropped_ipp_windows_keep_full_attachment_storage() {
    let screen = Viewport {
        screen: 0,
        render_target: -1,
        window: [10, 20, 630, 340],
    };
    let pool = Pool::new(screen, std::array::from_fn(|i| 100 + i as i32));
    assert_eq!(
        attachment_dimensions(&pool, screen, [640, 360]).unwrap(),
        [640, 360]
    );
    for i in 0..COUNT {
        let mut viewport = pool.viewport(i).unwrap();
        viewport.window = [1, 2, 8, 8];
        let expected = DIMENSIONS[pool.targets[i].size].map(|v| v as u32);
        assert_eq!(
            attachment_dimensions(&pool, viewport, [640, 360]).unwrap(),
            expected
        );
    }
    assert!(
        attachment_dimensions(
            &pool,
            Viewport {
                render_target: 999,
                ..screen
            },
            [640, 360]
        )
        .is_err()
    );
}
