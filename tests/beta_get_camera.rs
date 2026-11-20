use kkrieger_oxide::{
    collision::{connectivity, world::World},
    game::player::Motion,
    kx::Layout,
};
#[test]
fn released_get_camera_matches_native_euler_and_zoom_placement() {
    let mut words = include_bytes!("fixtures/beta-get-camera.bin")
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
        let mut float = || f32::from_bits(words.next().unwrap());
        motion.collider.position = std::array::from_fn(|_| float());
        motion.look = float();
        motion.direction = float();
        let zoom = words.next().unwrap() != 0;
        motion.camera_position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        for (field, actual) in motion.camera(zoom).into_iter().flatten().enumerate() {
            let expected = words.next().unwrap();
            assert_eq!(actual.to_bits(), expected, "case{case} matrix{field}");
        }
        // ZoomX, ZoomY, four untouched environment words, then Orthogonal.
        let tail: [u32; 7] = std::array::from_fn(|_| words.next().unwrap());
        let one = 1f32.to_bits();
        assert_eq!(
            tail,
            [one, one, 0xc0de0012, 0xc0de0013, 0xc0de0014, 0xc0de0015, 0],
            "case{case}"
        );
    }
    assert!(words.next().is_none());
}
