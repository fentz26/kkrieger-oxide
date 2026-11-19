use kkrieger_oxide::game::beta_monster::projectile::Point;
#[test]
fn point_projectile_spawn_matches_released_physic_handler() {
    let mut words = include_bytes!("fixtures/beta-point-projectile.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let data: [u32; 12] = std::array::from_fn(|_| words.next().unwrap());
        let matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let cell = words.next().unwrap();
        let event = words.next().unwrap();
        let skip = words.next().unwrap();
        let point = Point::new(
            &data,
            matrix,
            (cell != 0).then_some(7),
            (event != 0).then_some(77),
        )
        .unwrap();
        assert_eq!(point.flags, words.next().unwrap());
        assert_eq!(
            point.event,
            (data[0] & 0x20 != 0 && event != 0).then_some(77)
        );
        assert_eq!(point.particle.cell, (cell != 0).then_some(7));
        let p = &point.particle;
        let mut raw = [0; 32];
        raw[..7].copy_from_slice(&p.flags);
        raw[7] = cell;
        raw[8] = if point.skip_player { skip } else { 0 };
        raw[9] = if point.event.is_some() { event } else { 0 };
        raw[10] = p.damping.to_bits();
        raw[11] = p.weight.to_bits();
        raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
        for (out, value) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
            *out = value.to_bits();
        }
        for (field, actual) in raw.into_iter().enumerate() {
            assert_eq!(actual, words.next().unwrap(), "case {case} field {field}");
        }
    }
    assert!(words.next().is_none());
    assert!(Point::new(&[], kkrieger_oxide::mesh::identity(), None, None).is_err());
    let mut data = [0; 12];
    data[0] = 1;
    assert!(Point::new(&data, kkrieger_oxide::mesh::identity(), None, None).is_err());
}
