use kkrieger_oxide::game::beta_monster::body::Particle;
#[test]
fn standalone_particle_initialization_matches_released_code() {
    let mut words = include_bytes!("fixtures/beta-particle-init.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let speed = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let token = words.next().unwrap();
        let particle = Particle::new(position, speed, (token != 0).then_some(7));
        assert_eq!(particle.cell, (token != 0).then_some(7));
        let mut raw = [0; 32];
        raw[..7].copy_from_slice(&particle.flags);
        raw[7] = token;
        raw[10] = particle.damping.to_bits();
        raw[11] = particle.weight.to_bits();
        raw[12..16].copy_from_slice(&particle.force.map(f32::to_bits));
        for (out, value) in raw[16..]
            .iter_mut()
            .zip(particle.vectors.into_iter().flatten())
        {
            *out = value.to_bits();
        }
        for (field, actual) in raw.into_iter().enumerate() {
            assert_eq!(actual, words.next().unwrap(), "case {case} field {field}");
        }
    }
    assert!(words.next().is_none());
}
