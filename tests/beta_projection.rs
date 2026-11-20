use kkrieger_oxide::scene::camera::Environment;
#[test]
fn released_projection_matches_native_routine_in_every_mode() {
    let mut words = include_bytes!("fixtures/beta-projection.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v));
    assert_eq!(words.next(), Some(512));
    let sentinel: [[f32; 4]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| f32::from_bits(0xc0de_0000 + (i * 4 + j) as u32))
    });
    for case in 0..512 {
        let orthogonal = words.next().unwrap() as i32;
        let mut float = || f32::from_bits(words.next().unwrap());
        let e = Environment {
            zoom_x: float(),
            zoom_y: float(),
            center_x: float(),
            center_y: float(),
            near_clip: float(),
            far_clip: float(),
            orthogonal,
            released_projection: true,
            ..Default::default()
        };
        let viewport = [words.next().unwrap() as i32, words.next().unwrap() as i32];
        let mut actual = sentinel;
        e.released_projection_into(viewport, &mut actual);
        for (field, value) in actual.into_iter().flatten().enumerate() {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case{case} mode{orthogonal} field{field}"
            );
        }
        if orthogonal == 0 {
            assert_eq!(
                e.projection(viewport).unwrap().map(|r| r.map(f32::to_bits)),
                actual.map(|r| r.map(f32::to_bits)),
                "case{case}"
            );
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn execution_cameras_select_projection_by_document_layout() {
    use kkrieger_oxide::{kx::Layout, scene::viewport::Parameters};
    let beta = Environment::for_layout(Layout::Beta2004);
    let development = Environment::for_layout(Layout::Legacy3383);
    assert!(beta.released_projection && !development.released_projection);
    assert_eq!(
        beta.projection([640, 480]).unwrap()[3],
        [0., 0., -0.125, 0.]
    );
    assert_eq!(
        development.projection([640, 480]).unwrap()[3],
        [-1. / 640., 1. / 480., -0.125, 0.]
    );
    let mut words = [0u32; 25];
    words[1] = 4; // camera from game
    let parameters = Parameters::from_words(&words).unwrap();
    for game in [beta, development] {
        let camera = parameters.camera(&game, 1., None);
        assert_eq!(camera.released_projection, game.released_projection);
    }
}
