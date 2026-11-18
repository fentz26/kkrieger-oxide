use kkrieger_oxide::game::beta_monster::body::Body;
#[test]
fn particle_and_constraint_state_matches_released_initializer() {
    let bytes = include_bytes!("fixtures/beta-collider.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let cell_token = words.next().unwrap();
        let kind = words.next().unwrap() as i32;
        let body_token = words.next().unwrap();
        let cell = (cell_token != 0).then_some(7);
        let body = Body::new(position, cell, kind);
        for (particle, state) in body.particles.iter().enumerate() {
            assert_eq!(state.cell, cell);
            let mut raw = [0; 32];
            raw[..7].copy_from_slice(&state.flags);
            raw[7] = cell_token;
            raw[8] = body_token + 0x348;
            raw[11] = state.weight.to_bits();
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
                body_token + state.particles[0] as u32 * 128,
                body_token + state.particles[1] as u32 * 128,
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
#[test]
fn constraint_updates_match_released_x86() {
    let bytes = include_bytes!("fixtures/beta-constraint.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(512));
    for case in 0..512 {
        let a = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let b = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let rest = f32::from_bits(words.next().unwrap());
        let alias = words.next().unwrap() != 0;
        let mut body = Body::new([0., 0., 0., 1.], None, 3);
        body.particles[0].vectors[0] = a;
        body.particles[1].vectors[0] = b;
        body.constraints[0].rest_length = rest;
        if alias {
            body.constraints[0].particles = [0, 0];
        }
        body.solve_constraint(0).unwrap();
        for (field, value) in body.particles[0].vectors[0]
            .into_iter()
            .chain(body.particles[1].vectors[0])
            .enumerate()
        {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case {case} field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn invalid_constraint_links_do_not_modify_body() {
    let mut body = Body::new([1., 2., 3., 1.], Some(7), 3);
    let before = body.clone();
    assert_eq!(
        body.solve_constraint(10),
        Err("constraint index out of range")
    );
    assert_eq!(body, before);
    body.constraints[0].particles[1] = 5;
    let before = body.clone();
    assert_eq!(
        body.solve_constraint(0),
        Err("constraint particle index out of range")
    );
    assert_eq!(body, before);
}
