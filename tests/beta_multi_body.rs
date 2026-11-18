use kkrieger_oxide::game::beta_monster::{
    body::Body,
    hit::{HitRegistry, Shot},
    physics::Settings,
    runtime::{BodyRegistration, CollisionRuntime},
    subtractive::CandidateWorld,
    world_cast::AdditiveCell,
};
#[test]
fn global_pass_matches_original_registered_bodies() {
    let bytes = include_bytes!("fixtures/beta-multi-body.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(24));
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
    let settings = Settings {
        gravity: 0.02,
        gravity_player: 0.01,
        damping_player_air: 0.,
        damping_player_ground: 0.,
        fly_mode: 0,
        on_ground: false,
    };
    for case in 0..24 {
        let n = words.next().unwrap() as usize;
        let passes = words.next().unwrap();
        let mut bodies = Vec::new();
        let mut expected = Vec::new();
        let mut addresses = Vec::new();
        let mut ids = Vec::new();
        let mut hits = HitRegistry::default();
        for b in 0..n {
            let address = words.next().unwrap();
            addresses.push(address);
            let position = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
            let mut body = Body::new(position, None, 3);
            for p in &mut body.particles {
                p.force = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
            }
            let raw: [[u32; 32]; 5] =
                std::array::from_fn(|_| std::array::from_fn(|_| words.next().unwrap()));
            expected.push(raw);
            let identity = std::array::from_fn(|i| b * 5 + i);
            for (i, p) in body.particles.iter().enumerate() {
                hits.shots.insert(
                    identity[i],
                    Shot {
                        kind: p.flags[3],
                        event: None,
                    },
                );
            }
            ids.push(BodyRegistration {
                particles: identity,
                skip_cell: None,
            });
            bodies.push(body);
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
            particle_id: 999,
            skip_cell: None,
            monster_mode: false,
            interrupted: false,
            error: None,
        };
        let mut calls = 0;
        for pass in 0..passes {
            runtime
                .step_bodies(&mut bodies, &ids, settings, |_, bodies| {
                    calls += 1;
                    if pass == 0 {
                        assert!(bodies.iter().all(|b| b.particles[0].vectors[0][1] < 0.));
                        assert!(bodies.iter().all(|b| b.particles[0].force != [0.; 4]));
                    }
                    Ok(())
                })
                .unwrap();
            assert_eq!(runtime.particle_id, 999);
        }
        assert_eq!(calls, passes);
        for (b, body) in bodies.iter().enumerate() {
            for (i, p) in body.particles.iter().enumerate() {
                let mut raw = [0; 32];
                raw[..7].copy_from_slice(&p.flags);
                raw[7] = if p.cell.is_some() { 0x3002000 } else { 0 };
                raw[8] = addresses[b] + 0x348;
                raw[10] = p.damping.to_bits();
                raw[11] = p.weight.to_bits();
                raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
                for (out, v) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
                    *out = v.to_bits();
                }
                assert_eq!(raw, expected[b][i], "case {case} body {b} particle {i}");
            }
        }
        if case == 1 {
            let before = bodies.clone();
            let mut invalid = ids.clone();
            invalid[1].particles[4] = 9999;
            assert_eq!(
                runtime.step_bodies(&mut bodies, &invalid, settings, |_, _| panic!(
                    "unexpected dynamic phase"
                )),
                Err("shot identity missing")
            );
            assert_eq!(
                bodies, before,
                "all bodies validated before first relaxation"
            );
            assert_eq!(
                runtime.step_bodies(&mut bodies, &ids[..1], settings, |_, _| panic!(
                    "unexpected dynamic phase"
                )),
                Err("body registration count differs")
            );
            bodies[0].particles[0].force = [0.01, 0., 0., 0.];
            assert_eq!(
                runtime.step_bodies(&mut bodies, &ids, settings, |_, _| Err("dynamic failed")),
                Err("dynamic failed")
            );
            assert_eq!(runtime.particle_id, 999);
            assert_eq!(runtime.skip_cell, None);
            assert_eq!(bodies[0].particles[0].force, [0.01, 0., 0., 0.]);
        }
    }
    assert!(words.next().is_none());
}
