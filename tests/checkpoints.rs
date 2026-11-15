use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{State, monster::Health, player::Motion},
    mesh::identity,
};
use std::rc::Rc;
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
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn check_float(&mut self, v: f32) {
        assert_eq!(v.to_bits(), self.word(), "{v}");
    }
    fn check_vector(&mut self, v: [f32; 4]) {
        for x in v {
            self.check_float(x);
        }
    }
    fn placement(&mut self, m: &Motion) {
        self.check_vector(m.collider.position);
        self.check_vector(m.old_position);
        assert_eq!(m.collider.cell, self.word() as usize);
        self.check_float(m.collider.radius);
        self.check_float(m.direction);
        self.check_float(m.look);
        self.check_float(m.last_camera_y);
        self.check_vector(m.camera_position);
        let hit = m.hit_cell.borrow();
        for matrix in [hit.matrix0, hit.matrix1] {
            for v in matrix {
                self.check_vector(v);
            }
        }
        assert_eq!(m.jump_timer, self.word() as i32);
        self.check_float(m.speed_up);
        assert_eq!(m.on_ground, self.word() != 0);
    }
}
#[test]
fn original_checkpoint_registration_continue_and_placement() {
    let mut r = Reader(include_bytes!("fixtures/checkpoints.bin"));
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let cell = Cell::new(identity(), [20.; 3], 0);
        let world = World {
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
        };
        let mut game = State::default();
        let mut switches = [0; 256];
        assert_eq!(r.word(), 16);
        for i in 0..16 {
            let valid = r.word() != 0;
            switches[32 + i] = r.word() as u8;
            game.checkpoints[i].active = r.word() as i32;
            let position = [r.float(), r.float(), r.float()];
            let direction = r.float();
            if valid {
                game.register_checkpoint(i, position, direction).unwrap();
            }
        }
        game.parameters.start_position = [1., 2., 3., 1.];
        game.inventory.amounts[0] = case;
        game.inventory.amounts[4] = if case % 2 != 0 { 250 } else { 5 };
        for (life, state) in [(10, 4), (0, 1), (-1, 3)] {
            let actor = game.monsters.allocate().unwrap();
            actor.borrow_mut().health = Health {
                life,
                state,
                armor: 0,
                kill_switch: 0,
            };
            let id = actor.borrow().id;
            game.monsters.register(id).unwrap();
        }
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        let handle = Rc::clone(&motion.hit_cell);
        motion.jump_timer = 7;
        motion.speed_up = 0.2;
        motion.on_ground = true;
        game.continue_game(&mut motion, &world, &switches).unwrap();
        r.placement(&motion);
        assert_eq!(game.inventory.amounts[0], r.word() as i32);
        assert_eq!(game.inventory.amounts[4], r.word() as i32);
        for monster in game.monsters.active() {
            assert_eq!(monster.borrow().health.state, r.word() as i32);
        }
        for checkpoint in game.checkpoints {
            assert_eq!(checkpoint.valid, r.word() != 0);
            assert_eq!(checkpoint.active, r.word() as i32);
            r.check_vector(checkpoint.position);
            r.check_float(checkpoint.direction);
        }
        let index = r.word() as usize;
        let calls = r.word();
        let sound = game.respawn_at(index, &mut motion, &world).unwrap();
        assert_eq!(u32::from(sound.is_some()), calls);
        if let Some(sound) = sound {
            assert_eq!(sound.slot, 11);
            assert_eq!(sound.volume, 1.);
        }
        r.placement(&motion);
        let requested = r.vector();
        motion.place(&world, requested, 0.25, -0.5).unwrap();
        r.placement(&motion);
        assert!(Rc::ptr_eq(&handle, &motion.hit_cell));
    }
    assert!(r.0.is_empty());
}

#[test]
fn scene_checkpoint_retains_authored_coordinates_and_active_state() {
    use kkrieger_oxide::{
        kx::{self, Class},
        scene::exec::Execution,
    };
    let mut doc = kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.classes = vec![
        Class {
            id: 0xc3,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0x13,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = [1f32, 1., 1., 0., 0., 0., 90., 80., 70.]
        .map(f32::to_bits)
        .to_vec();
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![
        2f32.to_bits(),
        3f32.to_bits(),
        4f32.to_bits(),
        7,
        0.25f32.to_bits(),
    ];
    let mut execution = Execution::new(&doc);
    execution.game.checkpoints[7].active = 1;
    execution.frame(&doc, 0).unwrap();
    let checkpoint = execution.game.checkpoints[7];
    assert!(checkpoint.valid);
    assert_eq!(checkpoint.active, 1);
    assert_eq!(checkpoint.position, [2., 3., 4., 1.]);
    assert_eq!(checkpoint.direction, std::f32::consts::FRAC_PI_2);
    assert!(execution.game.register_checkpoint(16, [0.; 3], 0.).is_err());
}

#[test]
fn original_restart_resets_gameplay_state_and_camera_uses_resolved_position() {
    use kkrieger_oxide::game::zones;
    let mut r = Reader(include_bytes!("fixtures/restart-camera.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 4);
    for case in 0..64 {
        let cell = Cell::new(identity(), [20.; 3], 0);
        let mut world = World {
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
        };
        // Restart traverses zones in registration order, independent of geometry.
        for _ in 0..3 {
            world.zones.push(world.entries.len());
            world.entries.push(Entry {
                cell: Cell::new(identity(), [1.; 3], 2),
                sector: None,
                events: [None, None],
                logic: Logic {
                    code: 0,
                    condition: 0,
                    value: 0,
                    output: 0,
                    events: [None, None],
                },
            });
            world.faces.push(None);
        }
        let mut game = State {
            reset_by_operator: false,
            ..State::default()
        };
        game.parameters.start_position = r.vector();
        let mut switches = std::array::from_fn(|_| r.word() as u8);
        game.inventory.amounts[0] = case;
        game.inventory.amounts[4] = 5;
        for (life, state) in [(10, 4), (0, 1), (-1, 3)] {
            let actor = game.monsters.allocate().unwrap();
            actor.borrow_mut().health = Health {
                life,
                state,
                armor: 0,
                kill_switch: 0,
            };
            let id = actor.borrow().id;
            game.monsters.register(id).unwrap();
        }
        let mut zones = zones::Runtime::new(&world);
        for timer in &mut zones.respawn {
            *timer = r.word() as i32;
        }
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        let handle = Rc::clone(&motion.hit_cell);
        motion.jump_timer = 7;
        motion.speed_up = 0.2;
        motion.on_ground = true;
        let reset = game
            .restart(&mut motion, &world, &mut switches, &mut zones)
            .unwrap();
        assert_eq!(reset, world.zones);
        assert_eq!(game.reset_by_operator, r.word() != 0);
        r.placement(&motion);
        assert_eq!(game.inventory.amounts[0], r.word() as i32);
        assert_eq!(game.inventory.amounts[4], r.word() as i32);
        for monster in game.monsters.active() {
            assert_eq!(monster.borrow().health.state, r.word() as i32);
        }
        for value in switches {
            assert_eq!(u32::from(value), r.word());
        }
        for (i, &timer) in zones.respawn.iter().enumerate() {
            assert_eq!(timer, r.word() as i32);
            assert_eq!(r.word(), u32::from(i == 1));
            assert_eq!(r.word(), u32::from(i == 0));
        }
        assert!(Rc::ptr_eq(&handle, &motion.hit_cell));
        for _ in 0..4 {
            motion.collider.position = r.vector();
            motion.direction = r.float();
            let look = r.float();
            motion.set_orientation(look, false);
            game.parameters.damp_camera = r.float();
            game.parameters.camera_offset = r.float();
            let zoom = r.word() != 0;
            motion.update_camera(&game.parameters);
            r.check_float(motion.last_camera_y);
            r.check_vector(motion.camera_position);
            for v in motion.camera(zoom) {
                r.check_vector(v);
            }
            assert_eq!(r.float(), 1.);
            assert_eq!(r.float(), 1.);
            assert_eq!(r.word(), 0);
        }
    }
    assert!(r.0.is_empty());
}
