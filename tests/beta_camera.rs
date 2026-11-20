use kkrieger_oxide::{
    collision::{connectivity, world::World},
    game::{Parameters, player::Motion},
    kx::Layout,
};
#[test]
fn released_camera_smoothing_matches_native_x87_tail() {
    let mut words = include_bytes!("fixtures/beta-camera.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v));
    assert_eq!(words.next(), Some(512));
    let world = World {
        entries: vec![],
        adds: vec![],
        subs: vec![],
        zones: vec![],
        faces: vec![],
        connectivity: connectivity::connect(&[]),
    };
    let mut motion =
        Motion::spawn_for_layout(&world, [0., 0., 0., 1.], 0., Layout::Beta2004).unwrap();
    for case in 0..512 {
        motion.collider.position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        motion.last_camera_y = f32::from_bits(words.next().unwrap());
        let parameters = Parameters {
            damp_camera: f32::from_bits(words.next().unwrap()),
            camera_offset: f32::from_bits(words.next().unwrap()),
            ..Parameters::default()
        };
        let steps = words.next().unwrap();
        for step in 0..steps {
            motion.update_camera(&parameters);
            for (field, actual) in [motion.last_camera_y]
                .into_iter()
                .chain(motion.camera_position)
                .enumerate()
            {
                let expected = words.next().unwrap();
                assert_eq!(actual.to_bits(), expected, "case{case} step{step}/{field}");
            }
        }
    }
    assert!(words.next().is_none());
}
