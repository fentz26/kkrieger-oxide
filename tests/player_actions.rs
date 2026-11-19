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
