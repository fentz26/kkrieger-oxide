use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{Inventory, Parameters, player::Motion},
    mesh::identity,
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
#[test]
fn original_jump_and_damage_preserve_gating_armor_and_sound_order() {
    let cells = vec![Cell::new(identity(), [10.; 3], 0)];
    let graph = connectivity::connect(&cells);
    let world = World {
        entries: cells
            .into_iter()
            .map(|cell| Entry {
                cell,
                sector: None,
                events: [None, None],
                logic: Logic {
                    code: 0,
                    condition: 0,
                    value: 0,
                    output: 0,
                    events: [None, None],
                },
            })
            .collect(),
        adds: vec![0],
        subs: vec![],
        zones: vec![],
        connectivity: graph,
        faces: vec![None],
    };
    let mut r = Reader(include_bytes!("fixtures/player-actions.bin"));
    assert_eq!(r.word(), 256);
    assert_eq!(r.word(), 4);
    for case in 0..256 {
        let mut inventory = Inventory::default();
        inventory.amounts[0] = r.word() as i32;
        inventory.amounts[1] = r.word() as i32;
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        for call in 0..4 {
            let hits = r.word() as i32;
            motion.on_ground = r.word() != 0;
            motion.jump_timer = r.word() as i32;
            motion.speed_up = r.float();
            let parameters = Parameters {
                jump_force: r.float(),
                ..Default::default()
            };
            let jump = motion.jump(&parameters);
            let damage = inventory.hit(hits);
            let sounds: Vec<_> = jump.into_iter().chain(damage).collect();
            assert_eq!(inventory.amounts[0], r.word() as i32, "{case}/{call}");
            assert_eq!(inventory.amounts[1], r.word() as i32);
            assert_eq!(motion.jump_timer, r.word() as i32);
            assert_eq!(motion.speed_up.to_bits(), r.word());
            assert_eq!(sounds.len(), r.word() as usize);
            for sound in sounds {
                assert_eq!(sound.slot, r.word() as i32);
                assert_eq!(sound.volume.to_bits(), r.word());
                assert_eq!(sound.near_distance.to_bits(), r.word());
                assert_eq!(sound.far_distance.to_bits(), r.word());
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn released_player_damage_matches_native_signed_arithmetic_and_audio_calls() {
    let mut r = Reader(include_bytes!("fixtures/beta-player-hit.bin"));
    assert_eq!(r.word(), 1984);
    for case in 0..1984 {
        let mut inventory = Inventory::default();
        inventory.amounts[0] = r.word() as i32;
        inventory.amounts[1] = r.word() as i32;
        let hits = r.word() as i32;
        let sound = inventory.hit(hits);
        assert_eq!(inventory.amounts[0], r.word() as i32, "life case {case}");
        assert_eq!(inventory.amounts[1], r.word() as i32, "armor case {case}");
        assert_eq!(
            usize::from(sound.is_some()),
            r.word() as usize,
            "audio case {case}"
        );
        if let Some(sound) = sound {
            assert_eq!(sound.slot, r.word() as i32);
            assert_eq!(sound.volume.to_bits(), r.word());
            assert_eq!(r.word(), 0f32.to_bits()); // Native sample start offset.
            assert_eq!(r.word(), 44100); // Native playback frequency.
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn released_placement_preserves_outside_position_and_checkpoint_hit_identity() {
    let cells = vec![Cell::new(identity(), [10.; 3], 0)];
    let world = World {
        connectivity: connectivity::connect(&cells),
        entries: cells
            .into_iter()
            .map(|cell| Entry {
                cell,
                sector: None,
                events: [None, None],
                logic: Logic {
                    code: 0,
                    condition: 0,
                    value: 0,
                    output: 0,
                    events: [None, None],
                },
            })
            .collect(),
        adds: vec![0],
        subs: vec![],
        zones: vec![],
        faces: vec![None],
    };
    let outside = [100., -30., 20., 1.];
    let mut beta =
        Motion::spawn_for_layout(&world, outside, 0.25, kkrieger_oxide::kx::Layout::Beta2004)
            .unwrap();
    assert_eq!(beta.collider.position, outside);
    assert_eq!(beta.collider.cell, usize::MAX);
    assert_eq!(beta.camera_position, outside);
    let hit = std::rc::Rc::clone(&beta.hit_cell);
    beta.jump_timer = 7;
    beta.place(&world, [0., 1., 0., 1.], 0.5, 0.1).unwrap();
    assert_eq!(beta.collider.cell, 0);
    assert_eq!(beta.jump_timer, 7);
    assert!(std::rc::Rc::ptr_eq(&hit, &beta.hit_cell));
    beta.place(&world, outside, 0., 0.).unwrap();
    assert_eq!(beta.collider.position, outside);
    assert_eq!(beta.collider.cell, usize::MAX);
    let development = Motion::spawn(&world, outside, 0.).unwrap();
    assert_eq!(development.collider.position[..3], [0.; 3]);
    assert_eq!(development.collider.cell, 0);
}

#[test]
fn released_player_body_survives_frames_and_rebuilds_on_placement() {
    let cells = vec![
        Cell::new(identity(), [1.; 3], 1),
        Cell::new(identity(), [10.; 3], 0),
    ];
    let world = World {
        connectivity: connectivity::connect(&cells),
        entries: cells
            .into_iter()
            .map(|cell| Entry {
                cell,
                sector: None,
                events: [None, None],
                logic: Logic {
                    code: 0,
                    condition: 0,
                    value: 0,
                    output: 0,
                    events: [None, None],
                },
            })
            .collect(),
        adds: vec![1],
        subs: vec![0],
        zones: vec![],
        faces: vec![None, None],
    };
    let position = [2., 1., 0., 1.];
    let mut motion =
        Motion::spawn_for_layout(&world, position, 0., kkrieger_oxide::kx::Layout::Beta2004)
            .unwrap();
    assert_eq!(motion.collider.cell, 1);
    let player = motion.released_player.as_mut().unwrap();
    assert_eq!(
        player.physics[0].0,
        kkrieger_oxide::game::beta_monster::body::Body::new(position, Some(0), 1)
    );
    assert_eq!(player.physics[0].1.particles, [0, 1, 2, 3, 4]);
    player.physics[0].0.particles[0].force[0] = 3.;
    assert_eq!(
        motion.released_player.as_ref().unwrap().physics[0]
            .0
            .particles[0]
            .force[0],
        3.
    );
    motion.jump_timer = 8;
    motion.place(&world, [3., 2., 1., 1.], 0.25, 0.).unwrap();
    assert_eq!(motion.jump_timer, 8);
    assert_eq!(
        motion.released_player.as_ref().unwrap().physics[0].0,
        kkrieger_oxide::game::beta_monster::body::Body::new([3., 2., 1., 1.], Some(0), 1)
    );
    motion.place(&world, [100., 0., 0., 1.], 0., 0.).unwrap();
    assert!(
        motion.released_player.as_ref().unwrap().physics[0]
            .0
            .particles
            .iter()
            .all(|p| p.cell.is_none())
    );
    assert!(
        Motion::spawn(&world, position, 0.)
            .unwrap()
            .released_player
            .is_none()
    );
}

#[test]
fn released_motion_input_updates_persistent_particles_and_control_fields() {
    let world = World {
        entries: vec![],
        adds: vec![],
        subs: vec![],
        zones: vec![],
        connectivity: connectivity::connect(&[]),
        faces: vec![],
    };
    let mut motion = Motion::spawn_for_layout(
        &world,
        [2., 1., 0., 1.],
        0.,
        kkrieger_oxide::kx::Layout::Beta2004,
    )
    .unwrap();
    motion.matrix = identity();
    motion.accel_side = 0.1;
    motion.speed_up = 0.02;
    motion.jump_timer = 3;
    motion.off_ground_time = 0.5;
    motion.step_timer = 2.;
    motion.step_foot = 1;
    let parameters = Parameters {
        gravity: 0.01,
        stair_height: 1.,
        damp_height: 0.2,
        ..Default::default()
    };
    let before = motion.released_player.as_ref().unwrap().physics[0]
        .0
        .clone();
    let sound = motion.prepare_released(&parameters, 2, 0.5).unwrap();
    assert_eq!(sound.unwrap().slot, 5);
    assert_eq!(motion.jump_timer, 2);
    assert_eq!(motion.speed_up, 0.);
    assert!(motion.on_ground);
    assert_eq!(motion.off_ground_time, 0.);
    assert_eq!(motion.step_timer, 0.);
    assert_eq!(motion.step_foot, 0);
    let body = &motion.released_player.as_ref().unwrap().physics[0].0;
    assert!(body.particles[0].vectors[0][0] > 2.);
    assert!(body.particles[0].vectors[0][1] > 1.);
    assert_eq!(body.particles[0].vectors[2], before.particles[0].vectors[2]);
    assert_eq!(
        motion.collider.position,
        [2., 1., 0., 1.],
        "publication follows shared physics"
    );
}

#[test]
fn released_particle_publication_validates_cell_before_updating_motion() {
    let cells = vec![
        Cell::new(identity(), [1.; 3], 1),
        Cell::new(identity(), [10.; 3], 0),
    ];
    let world = World {
        connectivity: connectivity::connect(&cells),
        entries: cells
            .into_iter()
            .map(|cell| Entry {
                cell,
                sector: None,
                events: [None, None],
                logic: Logic {
                    code: 0,
                    condition: 0,
                    value: 0,
                    output: 0,
                    events: [None, None],
                },
            })
            .collect(),
        adds: vec![1],
        subs: vec![0],
        zones: vec![],
        faces: vec![None, None],
    };
    let mut motion = Motion::spawn_for_layout(
        &world,
        [2., 1., 0., 1.],
        0.,
        kkrieger_oxide::kx::Layout::Beta2004,
    )
    .unwrap();
    let main = &mut motion.released_player.as_mut().unwrap().physics[0]
        .0
        .particles[0];
    main.vectors[0] = [3., 2., 1., 0.];
    main.cell = Some(99);
    assert!(motion.publish_released(&world).is_err());
    assert_eq!(motion.collider.position, [2., 1., 0., 1.]);
    motion.released_player.as_mut().unwrap().physics[0]
        .0
        .particles[0]
        .cell = Some(0);
    assert_eq!(motion.publish_released(&world).unwrap(), [1., 1., 1., 0.]);
    assert_eq!(motion.collider.position, [3., 2., 1., 0.]);
    assert_eq!(motion.collider.cell, 1);
    motion.released_player.as_mut().unwrap().physics[0]
        .0
        .particles[0]
        .cell = None;
    motion.publish_released(&world).unwrap();
    assert_eq!(motion.collider.cell, usize::MAX);
}
