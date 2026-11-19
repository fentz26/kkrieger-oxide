use kkrieger_oxide::{
    collision::{
        Cell, connectivity, faces,
        world::{Entry, Logic, World},
    },
    game::{player::Motion, simulation::Simulation},
    kx::{Class, Document},
    mesh::identity,
    random::Random,
    scene::exec::Execution,
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

fn doc() -> Document<'static> {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![Class {
        id: 3,
        convention: 0,
        packing: &[],
    }];
    doc.operators.truncate(1);
    let op = &mut doc.operators[0];
    op.class = 0;
    op.animation = &[1];
    op.inputs.clear();
    op.links.clear();
    op.parameters.clear();
    doc
}
fn world() -> World {
    let mut floor_matrix = identity();
    floor_matrix[3][1] = -1.5;
    let cells = vec![
        Cell::new(identity(), [20.; 3], 0),
        Cell::new(floor_matrix, [20., 2., 20.], 1),
    ];
    let graph = connectivity::connect(&cells);
    let cell_faces = faces::create(&cells, 0, &graph.neighbors[0].adds).unwrap();
    World {
        connectivity: graph,
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
        subs: vec![1],
        zones: vec![],
        faces: vec![Some(cell_faces), None],
    }
}

#[test]
fn whole_original_player_tick_matches_native() {
    let doc = doc();
    let mut r = Reader(include_bytes!("fixtures/tick-player.bin"));
    assert_eq!(r.word(), 24);
    assert_eq!(r.word(), 6);
    for case in 0..24 {
        let mut execution = Execution::new(&doc);
        execution.game.inventory.amounts[0] = r.word() as i32;
        execution.game.inventory.amounts[4] = 5;
        execution.game.parameters.start_position = [1., 2., 3., 1.];
        let p = &mut execution.game.parameters;
        p.gravity = 0.01;
        p.damp_ground = 0.05;
        p.damp_air = 0.025;
        p.stair_height = 0.55;
        p.damp_camera = 0.25;
        p.camera_offset = 0.85;
        p.mouse_turn = 0.001;
        p.mouse_look = 0.002;
        p.weapon_cool[0] = 0.125;
        execution.game.weapon.current = 0;
        execution.game.weapon.next = 0;
        execution.game.weapon.timer = 0.25;
        let mut simulation = Simulation::new(
            world(),
            &mut execution,
            [0., 0., 0., 1.],
            0,
            [0; 2],
            Random::new(1),
        )
        .unwrap();
        simulation.motion.off_ground_time = if case % 2 != 0 { 0.5 } else { 0. };
        simulation.bind_scene(&mut execution, false);
        execution.frame(&doc, 0).unwrap();
        let mut options_changed = 0;
        for step in 0..6 {
            let slices = r.word() as i32;
            let now = r.word() as i32;
            let mouse = [r.word() as i32, r.word() as i32];
            execution.switches[1] = r.word() as u8;
            execution.switches[16] = 1;
            execution.switches[13] = r.word() as u8;
            execution.switches[12] = r.word() as u8;
            simulation.player_cell = if r.word() != 0 { Some(0) } else { None };
            execution.game.weapon.fire_key = r.word() != 0;
            execution.game.weapon.use_key = true;
            simulation.motion.accel_forward = r.float();
            simulation.motion.accel_side = r.float();
            let tick = simulation
                .tick(&mut execution, &doc, slices, now, mouse)
                .unwrap();
            options_changed += u32::from(tick.options_changed);
            assert_eq!(simulation.tick_count, r.word() as i32, "{case}/{step}");
            let was_options = execution.switches[1] == 9 || execution.switches[1] == 1;
            assert_eq!(was_options, r.word() != 0);
            assert_eq!(options_changed, r.word());
            assert_eq!(simulation.input.last_time, r.word() as i32);
            assert_eq!(
                simulation.input.last_mouse,
                [r.word() as i32, r.word() as i32]
            );
            r.placement(&simulation.motion);
            for v in simulation.motion.matrix {
                r.check_vector(v);
            }
            assert_eq!(execution.game.inventory.amounts[0], r.word() as i32);
            assert_eq!(execution.game.inventory.amounts[4], r.word() as i32);
            assert_eq!(execution.game.weapon.fire_key, r.word() != 0);
            assert_eq!(execution.game.weapon.use_key, r.word() != 0);
            r.check_float(execution.game.weapon.cooldown);
            r.check_float(execution.game.weapon.timer);
            r.check_float(simulation.motion.off_ground_time);
            assert_eq!(simulation.motion.step_foot, r.word() as i32);
            r.check_float(simulation.motion.step_timer);
            assert_eq!(simulation.player_cell.is_some(), r.word() != 0);
            assert_eq!(tick.listeners.len(), r.word() as usize);
            for listener in tick.listeners {
                for v in [
                    listener.position,
                    listener.velocity,
                    listener.up,
                    listener.forward,
                ] {
                    for x in v {
                        let expected = r.float();
                        assert!(
                            x.to_bits() == expected.to_bits() || x.is_nan() && expected.is_nan(),
                            "listener {case}/{step}: {x} vs {expected}"
                        );
                    }
                }
            }
            assert_eq!(tick.sounds.len(), r.word() as usize);
            for audio in tick.sounds {
                assert_eq!(audio.sound.slot, r.word() as i32);
                r.check_float(audio.sound.volume);
                r.check_vector(audio.position);
            }
        }
    }
    assert!(r.0.is_empty());
}

fn configured() -> (Document<'static>, Execution, Simulation) {
    let doc = doc();
    let mut execution = Execution::new(&doc);
    execution.game.inventory.amounts[0] = 100;
    execution.game.inventory.amounts[4] = 5;
    execution.game.parameters.gravity = 0.01;
    execution.game.parameters.stair_height = 0.55;
    execution.game.parameters.camera_offset = 0.85;
    execution.game.weapon.current = 0;
    execution.game.weapon.next = 0;
    execution.game.weapon.timer = 0.25;
    let simulation = Simulation::new(
        world(),
        &mut execution,
        [0., 0., 0., 1.],
        0,
        [0; 2],
        Random::new(1),
    )
    .unwrap();
    simulation.bind_scene(&mut execution, false);
    execution.frame(&doc, 0).unwrap();
    (doc, execution, simulation)
}
#[test]
fn splash_kills_before_the_actor_can_attack_in_the_same_tick() {
    use kkrieger_oxide::game::monster_actor::Context;
    let (doc, mut execution, mut simulation) = configured();
    let actor = execution.game.monsters.allocate().unwrap();
    let mut p = [0; 29];
    p[6] = 100f32.to_bits();
    p[7] = 10f32.to_bits();
    p[8] = 5f32.to_bits();
    p[9] = 8;
    p[10] = 5;
    p[12] = 10;
    p[13] = 1f32.to_bits();
    p[15] = 1f32.to_bits();
    p[20] = 1f32.to_bits();
    p[21] = 1f32.to_bits();
    p[22] = 1f32.to_bits();
    p[26] = 4;
    p[27] = u32::MAX;
    let mut parent = identity();
    parent[3] = [0., 0., 1., 1.];
    actor
        .borrow_mut()
        .initialize(
            &p,
            Context {
                world: &mut simulation.world.borrow_mut(),
                dynamic: &[],
                player_position: simulation.motion.collider.position,
                parent,
                spline: None,
                time: 0,
                time_reset: false,
                flags: None,
            },
        )
        .unwrap();
    actor.borrow_mut().health.state = 4;
    actor.borrow_mut().melee_timer = 1.;
    actor.borrow_mut().ranged_timer = 1.;
    execution.game.monsters.register(actor.borrow().id).unwrap();
    execution
        .dynamic_cells
        .push(std::rc::Rc::clone(&actor.borrow().hit_cell));
    execution.splash.enabled = true;
    execution.splash.position = [0., 0., 0., 1.];
    execution.splash.range = 5.;
    execution.splash.hits = 20;
    let tick = simulation
        .tick(&mut execution, &doc, 1, 10, [0; 2])
        .unwrap();
    assert_eq!(actor.borrow().health.life, 0);
    assert_eq!(actor.borrow().health.state, 1);
    assert_eq!(execution.game.inventory.amounts[0], 90);
    assert_eq!(tick.sounds.len(), 1);
    assert_eq!(tick.sounds[0].sound.slot, 10);
}
#[test]
fn player_fires_before_movement_and_projectile_advances_in_the_same_slice() {
    let (doc, mut execution, mut simulation) = configured();
    execution.game.events.shot[0] = Some(0);
    execution.game.weapon.fire_key = true;
    simulation.motion.accel_forward = 0.1;
    simulation.motion.look = 0.02 * std::f32::consts::TAU;
    simulation
        .tick(&mut execution, &doc, 1, 10, [0; 2])
        .unwrap();
    assert_eq!(simulation.events.shots.len(), 1);
    assert_eq!(simulation.events.shots[0].state.old_position[2], 1.25);
    assert_eq!(simulation.events.shots[0].state.matrix[3][2], 1.5);
    assert_eq!(simulation.motion.collider.position[2], 0.1);
}

#[test]
fn respawn_key_refreshes_the_tick_owner_collision_cell_and_mouse_baseline() {
    let (_, mut execution, mut simulation) = configured();
    simulation.player_cell = None;
    let armed = simulation.key(&mut execution, b'm' as u32, [3, 7]).unwrap();
    assert!(!armed.placed);
    assert!(simulation.player_cell.is_none());
    let placed = simulation.key(&mut execution, b'1' as u32, [5, 9]).unwrap();
    assert!(placed.placed);
    assert_eq!(simulation.player_cell, Some(0));
    assert_eq!(simulation.input.last_mouse, [5, 9]);
}

#[test]
fn preparation_installs_the_new_frame_clock_after_tick_and_sets_camera_aspect() {
    let (doc, mut execution, mut simulation) = configured();
    execution.current_time = 11;
    execution.last_time = 0;
    simulation
        .tick(&mut execution, &doc, 1, 20, [0; 2])
        .unwrap();
    assert_eq!(execution.current_time, 11);
    simulation
        .prepare_scene(&mut execution, 65536, 20, 1.5, false)
        .unwrap();
    assert_eq!(execution.last_time, 11);
    assert_eq!(execution.current_time, 20);
    assert_eq!(execution.time_delta, 9);
    assert_eq!(execution.time_slices, 1);
    assert!(!execution.time_reset);
    assert_eq!(execution.vm.variables[0], [1.; 4]);
    assert_eq!(execution.game_camera.space, simulation.motion.camera(false));
    assert_eq!(execution.game_camera.zoom_y, 1.5);
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.dynamic_cells.len(), 1);
    assert!(std::rc::Rc::ptr_eq(
        &execution.dynamic_cells[0],
        &simulation.motion.hit_cell
    ));
    execution.finish_events();
}

#[test]
fn released_main_tick_steps_points_and_synchronizes_owning_events() {
    use kkrieger_oxide::{
        collision::DynamicCell,
        game::beta_monster::{
            projectile::{PhysicsSlot, Point},
            registration::DynamicSlot,
        },
        scene::event::Event,
    };
    for reflect in [false, true] {
        let mut doc = doc();
        doc.layout = kkrieger_oxide::kx::Layout::Beta2004;
        let mut execution = Execution::new(&doc);
        execution.switches[1] = 0;
        execution.game.parameters.stair_height = 0.55;
        execution.game.parameters.weapon_flags[0] = if reflect { 2 } else { 0 };
        execution.game.events.explode[0][0] = Some(0);
        let mut simulation = Simulation::new(
            world(),
            &mut execution,
            [-5., 0., 0., 1.],
            0,
            [0, 0],
            Random::new(1),
        )
        .unwrap();
        simulation.tick_count = 19;
        let event = execution.events.insert(
            Event {
                start: 50,
                end: 100,
                ..Default::default()
            },
            true,
        );
        let mut words = [0; 12];
        words[0] = 0x20;
        words[1] = 2.5f32.to_bits();
        words[10] = 1f32.to_bits();
        words[11] = 8;
        let point = std::rc::Rc::new(std::cell::RefCell::new(
            Point::new(&words, identity(), Some(0), Some(event)).unwrap(),
        ));
        let mut matrix = identity();
        matrix[3][0] = 1.5;
        let wall = std::rc::Rc::new(std::cell::RefCell::new(DynamicCell::new(
            Cell::new(identity(), [1., 4., 4.], 1),
            matrix,
        )));
        wall.borrow_mut().cell.mode &= !0x20;
        execution.dynamic_cells = vec![simulation.motion.hit_cell.clone(), wall];
        execution.released_order = vec![DynamicSlot::Scene(0), DynamicSlot::Scene(1)];
        execution.released_points.push(point.clone());
        execution.released_physics_order.push(PhysicsSlot::Point(0));
        let result = simulation
            .tick(&mut execution, &doc, 1, 10, [0, 0])
            .unwrap();
        assert!(result.ran);
        assert!((0. ..1.).contains(&point.borrow().particle.vectors[0][0]));
        let effects = execution
            .events
            .events
            .iter()
            .filter(|(id, event_state)| **id != event && event_state.borrow().operator == Some(0))
            .count();
        assert_eq!(effects, 1);
        assert!(
            simulation
                .released_actors
                .as_ref()
                .unwrap()
                .hits
                .pending
                .is_empty()
        );
        let live = execution.events.events[&event].borrow();
        assert_eq!(live.shot_time, 20);
        assert_eq!(
            (live.start, live.end, live.select),
            if reflect { (50, 100, 0) } else { (0, 1, 1) }
        );
    }
}
