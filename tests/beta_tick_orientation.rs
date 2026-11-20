use kkrieger_oxide::{
    collision::{connectivity, world::World},
    game::{Parameters, player::Motion},
    kx::Layout,
};
#[test]
fn released_tick_player_and_hit_cell_matrices_match_native_euler() {
    let mut words = include_bytes!("fixtures/beta-tick-orientation.bin")
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
    let parameters = Parameters::default();
    for case in 0..512 {
        let look = f32::from_bits(words.next().unwrap());
        motion.direction = f32::from_bits(words.next().unwrap());
        let fly = words.next().unwrap() != 0;
        motion.collider.position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        motion.set_orientation(look, fly);
        assert_eq!(motion.look.to_bits(), look.to_bits());
        for (field, actual) in motion.matrix.into_iter().flatten().enumerate() {
            let expected = words.next().unwrap();
            assert_eq!(actual.to_bits(), expected, "case{case} player{field}");
        }
        motion.prepare_released(&parameters, 1, 0.5).unwrap();
        let hit = motion.hit_cell.borrow();
        assert_eq!(
            hit.matrix0[3].map(f32::to_bits),
            hit.matrix1[3].map(f32::to_bits),
            "case{case}"
        );
        for (field, actual) in hit.matrix1.into_iter().flatten().enumerate() {
            let expected = words.next().unwrap();
            assert_eq!(actual.to_bits(), expected, "case{case} hit{field}");
        }
    }
    assert!(words.next().is_none());
}
