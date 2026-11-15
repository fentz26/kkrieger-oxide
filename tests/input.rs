use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{State, input::Input, player::Motion},
    mesh::identity,
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
fn world() -> World {
    let cell = Cell::new(identity(), [20.; 3], 0);
    World {
        connectivity: connectivity::connect(std::slice::from_ref(&cell)),
        entries: vec![Entry {
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
        }],
        adds: vec![0],
        subs: vec![],
        zones: vec![],
        faces: vec![Some(vec![])],
    }
}
#[test]
fn original_key_handling_preserves_release_cheats_weapon_mapping_and_sounds() {
    let world = world();
    let mut r = Reader(include_bytes!("fixtures/input-keys.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 48);
    for case in 0..64 {
        let mut game = State::default();
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        let mut input = Input::new(0, [0; 2]);
        game.parameters.jump_force = 0.25;
        game.parameters.accel_forward = 0.01;
        game.parameters.accel_back = 0.005;
        game.parameters.accel_side = 0.0075;
        for i in 0..16 {
            let amount = r.int();
            let max = r.int();
            if i < 8 {
                game.inventory.amounts[i] = amount;
                game.inventory.maxima[i] = max;
            } else {
                game.inventory.weapons[i - 8] = amount;
                assert_eq!(max, 1);
            }
            if r.word() != 0 {
                game.register_checkpoint(i, [0.; 3], 0.).unwrap();
            }
        }
        let mut placements = 0;
        for tick in 0..48 {
            let key = r.word();
            let mouse = [r.int(), r.int()];
            motion.on_ground = r.word() != 0;
            motion.jump_timer = r.int();
            motion.speed_up = r.float();
            let result = input
                .key(key, mouse, &mut game, &mut motion, &world)
                .unwrap();
            placements += usize::from(result.placed);
            assert_eq!(result.handled, r.word() != 0, "{case}/{tick}");
            assert_eq!(input.last_key, r.word());
            assert_eq!(input.fly, r.word() != 0);
            for f in [motion.accel_forward, motion.accel_side, motion.speed_up] {
                assert_eq!(f.to_bits(), r.word());
            }
            assert_eq!(motion.jump_timer, r.int());
            assert_eq!(game.weapon.use_key, r.word() != 0);
            assert_eq!(game.weapon.fire_key, r.word() != 0);
            assert_eq!(game.weapon.next, r.word() as usize);
            assert_eq!(placements, r.word() as usize);
            assert_eq!(input.last_mouse, [r.int(), r.int()]);
            for value in game
                .inventory
                .amounts
                .into_iter()
                .chain(game.inventory.weapons)
            {
                assert_eq!(value, r.int(), "{case}/{tick}");
            }
            assert_eq!(result.sounds.len(), r.word() as usize);
            for sound in result.sounds {
                assert_eq!(sound.slot, r.int());
                for f in [sound.volume, sound.near_distance, sound.far_distance] {
                    assert_eq!(f.to_bits(), r.word());
                }
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn original_mouse_prefix_preserves_pause_cutoff_counters_and_fly_orientation() {
    let world = world();
    let mut r = Reader(include_bytes!("fixtures/input-look.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 8);
    for case in 0..128 {
        let now = r.int();
        let mut input = Input::new(now, [r.int(), r.int()]);
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        motion.direction = r.float();
        motion.look = r.float();
        let mut game = State::default();
        game.parameters.mouse_turn = r.float();
        game.parameters.mouse_look = r.float();
        let mut switches = [0; 256];
        for tick in 0..8 {
            let now = r.int();
            let mouse = [r.int(), r.int()];
            input.fly = r.word() != 0;
            switches[13] = r.word() as u8;
            switches[12] = r.word() as u8;
            let elapsed = now.wrapping_sub(input.last_time);
            assert_eq!(
                input.look(now, mouse, &game, &mut motion, &switches),
                elapsed
            );
            assert_eq!(input.last_time, r.int());
            assert_eq!(input.last_mouse, [r.int(), r.int()]);
            assert_eq!(motion.direction.to_bits(), r.word(), "{case}/{tick}");
            assert_eq!(motion.look.to_bits(), r.word(), "{case}/{tick}");
            for v in motion.matrix {
                for x in v {
                    assert_eq!(x.to_bits(), r.word(), "{case}/{tick}");
                }
            }
        }
    }
    assert!(r.0.is_empty());
}
