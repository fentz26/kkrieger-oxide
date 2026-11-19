use kkrieger_oxide::game::beta_monster::body::BoxBody;
#[test]
fn box_particles_and_constraints_match_released_initializer() {
    let mut words = include_bytes!("fixtures/beta-box-init.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let scale = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let speed = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let cell_token = words.next().unwrap();
        let cell = (cell_token != 0).then_some(7);
        let body = BoxBody::new(matrix, scale, speed, cell);
        for (particle, state) in body.particles.iter().enumerate() {
            assert_eq!(state.cell, cell);
            let mut raw = [0; 32];
            raw[..7].copy_from_slice(&state.flags);
            raw[7] = cell_token;
            raw[10] = state.damping.to_bits();
            raw[11] = state.weight.to_bits();
            raw[12..16].copy_from_slice(&state.force.map(f32::to_bits));
            for (out, value) in raw[16..]
                .iter_mut()
                .zip(state.vectors.into_iter().flatten())
            {
                *out = value.to_bits();
            }
            for (field, value) in raw.into_iter().enumerate() {
                assert_eq!(
                    value,
                    words.next().unwrap(),
                    "case {case} particle {particle} field {field}"
                );
            }
        }
        for (constraint, state) in body.constraints.iter().enumerate() {
            let raw = [
                state.flags[0],
                state.flags[1],
                state.particles[0] as u32,
                state.particles[1] as u32,
                state.rest_length.to_bits(),
            ];
            for (field, value) in raw.into_iter().enumerate() {
                assert_eq!(
                    value,
                    words.next().unwrap(),
                    "case {case} constraint {constraint} field {field}"
                );
            }
        }
    }
    assert!(words.next().is_none());
}
