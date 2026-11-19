use kkrieger_oxide::game::beta_monster::{
    body::BoxBody,
    hit::{HitRegistry, Shot},
    physics::Settings,
    runtime::CollisionRuntime,
    subtractive::CandidateWorld,
    world_cast::AdditiveCell,
};
#[test]
fn box_pass_matches_complete_released_physics() {
    let bytes = include_bytes!("fixtures/beta-box-pass.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(32));
    let planes = [
        [1., 0., 0., 2.],
        [-1., 0., 0., 2.],
        [0., 1., 0., 2.],
        [0., -1., 0., 2.],
        [0., 0., 1., 2.],
        [0., 0., -1., 2.],
    ];
    let adds = [AdditiveCell {
        planes: &planes,
        neighbors: &[],
        only_for_shot: false,
    }];
    let lists: [&[usize]; 1] = [&[]];
    for case in 0..32 {
        let passes = words.next().unwrap();
        let settings = Settings {
            gravity: f32::from_bits(words.next().unwrap()),
            gravity_player: f32::from_bits(words.next().unwrap()),
            damping_player_air: f32::from_bits(words.next().unwrap()),
            damping_player_ground: f32::from_bits(words.next().unwrap()),
            fly_mode: 0,
            on_ground: false,
        };
        let mut matrix = kkrieger_oxide::mesh::identity();
        matrix[3][0] = -0.5;
        let mut body = BoxBody::new(matrix, [1.; 3], [0.05, 0.01, 0., 0.], None);
        for particle in &mut body.particles {
            particle.flags[3] = 8;
            particle.weight = 1.;
        }
        for p in &mut body.particles {
            p.force = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        }
        let mut hits = HitRegistry::default();
        for (i, particle) in body.particles.iter().enumerate() {
            hits.shots.insert(
                i,
                Shot {
                    kind: particle.flags[3],
                    event: None,
                },
            );
        }
        let mut runtime = CollisionRuntime {
            adds: &adds,
            candidates: CandidateWorld {
                static_cells: &[],
                lists: &lists,
                dynamic_cells: &mut [],
            },
            zones: &mut [],
            zone_lists: &lists,
            hits: &mut hits,
            particle_id: 99,
            skip_cell: None,
            monster_mode: false,
            interrupted: false,
            error: None,
        };
        let mut dynamic_calls = 0;
        for _ in 0..passes {
            runtime
                .step_box(&mut body, [0, 1, 2, 3, 4, 5, 6, 7], settings, |_, _| {
                    dynamic_calls += 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(runtime.particle_id, 99);
        }
        assert_eq!(dynamic_calls, passes);
        for (i, p) in body.particles.iter().enumerate() {
            let expected: [u32; 32] = std::array::from_fn(|_| words.next().unwrap());
            let mut raw = [0; 32];
            raw[..7].copy_from_slice(&p.flags);
            raw[7] = if p.cell.is_some() { 0x3002000 } else { 0 };

            raw[10] = p.damping.to_bits();
            raw[11] = p.weight.to_bits();
            raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
            for (out, v) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
                *out = v.to_bits();
            }
            assert_eq!(raw, expected, "case {case} particle {i}");
        }
        if case == 0 {
            let before = body.clone();
            assert_eq!(
                runtime.step_box(
                    &mut body,
                    [0, 1, 2, 3, 4, 5, 6, 99],
                    settings,
                    |_, _| panic!("unexpected dynamic phase")
                ),
                Err("shot identity missing")
            );
            assert_eq!(body, before);
            body.constraints[15].particles = [8, 0];
            let invalid = body.clone();
            assert_eq!(
                runtime.step_box(
                    &mut body,
                    [0, 1, 2, 3, 4, 5, 6, 7],
                    settings,
                    |_, _| panic!("unexpected dynamic phase")
                ),
                Err("constraint particle index out of range")
            );
            assert_eq!(body, invalid);
            body = before;
            body.particles[0].force = [0.01, 0., 0., 0.];
            let result = runtime.step_box(&mut body, [0, 1, 2, 3, 4, 5, 6, 7], settings, |_, _| {
                Err("dynamic phase failed")
            });
            assert_eq!(result, Err("dynamic phase failed"));
            assert_eq!(runtime.particle_id, 99);
            assert_eq!(
                body.particles[0].force,
                [0.01, 0., 0., 0.],
                "force phase must not run after error"
            );
        }
    }
    assert!(words.next().is_none());
}
