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

#[test]
fn box_child_matrix_matches_released_particle_reconstruction() {
    let mut words = include_bytes!("fixtures/beta-box-matrix.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let mut body = BoxBody::new(kkrieger_oxide::mesh::identity(), [1.; 3], [0.; 4], None);
        for particle in &mut body.particles {
            particle.vectors[0] = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        }
        let matrix = body.matrix();
        for (field, value) in matrix.into_iter().flatten().enumerate() {
            assert_eq!(
                value.to_bits(),
                words.next().unwrap(),
                "case {case} matrix field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn scene_box_initializer_matches_released_physic_handler() {
    use kkrieger_oxide::game::beta_monster::projectile::BoxProjectile;
    let mut words = include_bytes!("fixtures/beta-box-projectile.bin")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let parameters = std::array::from_fn::<_, 12, _>(|_| words.next().unwrap());
        let matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let cell_token = words.next().unwrap();
        let event_token = words.next().unwrap();
        let skip_token = words.next().unwrap();
        let cell = (cell_token != 0).then_some(7);
        let event = (event_token != 0).then_some(99);
        let projectile = BoxProjectile::new(&parameters, matrix, cell, event).unwrap();
        assert_eq!(projectile.flags, words.next().unwrap());
        assert_eq!(projectile.skip_player, parameters[0] & 0x20 != 0);
        assert_eq!(
            projectile.event,
            if projectile.skip_player { event } else { None }
        );
        for (index, particle) in projectile.body.particles.iter().enumerate() {
            assert_eq!(particle.cell, cell);
            let mut raw = [0; 32];
            raw[..7].copy_from_slice(&particle.flags);
            raw[7] = cell_token;
            raw[8] = if projectile.skip_player {
                skip_token
            } else {
                0
            };
            raw[9] = if projectile.event.is_some() {
                event_token
            } else {
                0
            };
            raw[10] = particle.damping.to_bits();
            raw[11] = particle.weight.to_bits();
            raw[12..16].copy_from_slice(&particle.force.map(f32::to_bits));
            for (out, value) in raw[16..]
                .iter_mut()
                .zip(particle.vectors.into_iter().flatten())
            {
                *out = value.to_bits();
            }
            for (field, value) in raw.into_iter().enumerate() {
                assert_eq!(
                    value,
                    words.next().unwrap(),
                    "case {case} particle {index} field {field}"
                );
            }
        }
        for constraint in &projectile.body.constraints {
            for value in [
                constraint.flags[0],
                constraint.flags[1],
                constraint.particles[0] as u32,
                constraint.particles[1] as u32,
                constraint.rest_length.to_bits(),
            ] {
                assert_eq!(value, words.next().unwrap(), "case {case}");
            }
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn box_constraint_slots_match_native_physics() {
    let mut words = include_bytes!("fixtures/beta-constraint.bin")
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
        let mut body = BoxBody::new(kkrieger_oxide::mesh::identity(), [1.; 3], [0.; 4], None);
        let ai = case % 8;
        let bi = (ai + 1 + case % 7) % 8;
        let ci = case % 16;
        body.particles[ai].vectors[0] = a;
        body.particles[bi].vectors[0] = b;
        body.constraints[ci].particles = [ai, if alias { ai } else { bi }];
        body.constraints[ci].rest_length = rest;
        body.solve_constraint(ci).unwrap();
        for index in [ai, bi] {
            for value in body.particles[index].vectors[0] {
                assert_eq!(
                    value.to_bits(),
                    words.next().unwrap(),
                    "case {case} particle {index}"
                );
            }
        }
    }
    assert!(words.next().is_none());
}

#[test]
fn box_constraint_validation_preserves_particle_state() {
    let mut body = BoxBody::new(kkrieger_oxide::mesh::identity(), [1.; 3], [0.; 4], None);
    let before = body.particles.clone();
    assert_eq!(
        body.solve_constraint(16),
        Err("constraint index out of range")
    );
    body.constraints[15].particles = [7, 8];
    assert_eq!(
        body.solve_constraint(15),
        Err("constraint particle index out of range")
    );
    assert_eq!(body.particles, before);
}
