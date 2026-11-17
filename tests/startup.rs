use kkrieger_oxide::{
    collision::{connectivity, world::World},
    game::{State, root, simulation::Simulation},
    kx::Document,
    random::Random,
    scene::exec::Execution,
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn float(&mut self, n: f32) {
        assert_eq!(n.to_bits(), self.word());
    }
}
#[test]
fn original_startup_and_all_root_modes() {
    let mut r = Reader(include_bytes!("fixtures/startup.bin"));
    assert_eq!(r.word(), 1);
    let state = State::player_build();
    for n in state
        .inventory
        .amounts
        .into_iter()
        .chain(state.inventory.weapons)
    {
        assert_eq!(n, r.word() as i32);
    }
    for n in state.inventory.maxima.into_iter().chain([1; 8]) {
        assert_eq!(n, r.word() as i32);
    }
    for n in [
        state.weapon.current as u32,
        state.weapon.next as u32,
        u32::from(state.weapon.fire_key),
        u32::from(state.weapon.use_key),
    ] {
        assert_eq!(n, r.word());
    }
    r.float(state.weapon.cooldown);
    let p = &state.parameters;
    for n in [
        p.gravity,
        p.damp_ground,
        p.damp_air,
        p.damp_height,
        p.stair_height,
        p.eye_height,
        p.mouse_turn,
        p.mouse_look,
        p.accel_side,
        p.accel_forward,
        p.accel_back,
        p.jump_force,
        p.damp_camera,
        p.camera_offset,
    ] {
        r.float(n);
    }
    assert_eq!(u32::from(state.reset_by_operator), r.word());
    for n in root::switches() {
        assert_eq!(u32::from(n), r.word());
    }
    assert_eq!(r.word(), 0); // Original WasInOptions.
    assert_eq!(r.word(), 1); // Startup requests one platform options update.
    assert_eq!(r.word(), 256);
    for _ in 0..256 {
        let mut switches = std::array::from_fn(|_| r.word() as u8);
        let selected = root::select(&mut switches);
        assert_eq!(selected.index, r.word() as usize);
        assert_eq!(u32::from(selected.restart), r.word());
        assert_eq!(u32::from(selected.quit), r.word());
        for n in switches {
            assert_eq!(u32::from(n), r.word());
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn menu_world_gates_simulation_and_restart_preserves_inventory() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut e = Execution::new_player(&doc);
    assert_eq!(e.switches, root::switches());
    let world = World {
        entries: vec![],
        adds: vec![],
        subs: vec![],
        zones: vec![],
        faces: vec![],
        connectivity: connectivity::connect(&[]),
    };
    let mut sim =
        Simulation::new(world, &mut e, [0., 0., 0., 1.], 0, [0, 0], Random::new(1)).unwrap();
    assert!(sim.player_cell.is_none());
    assert!(!sim.tick(&mut e, &doc, 10, 100, [20, 30]).unwrap().ran);
    assert_eq!(sim.tick_count, 0);
    assert_eq!(e.game.inventory.amounts[0], 100);
    e.game.parameters.start_position = [1., 2., 3., 1.];
    e.game.inventory.amounts[0] = 20;
    e.game.inventory.amounts[4] = 5;
    e.switches[1] = 2;
    e.switches[40] = 1;
    e.switches[8] = 2;
    let s = sim.select_root(&mut e, [40, 50]).unwrap();
    assert_eq!(
        s,
        root::Selection {
            index: 2,
            quit: false,
            restart: true
        }
    );
    assert_eq!(e.switches[1], 0);
    assert_eq!(e.switches[40], 0);
    assert_eq!(e.switches[8], 2);
    assert!(e.game.reset_by_operator);
    assert!(sim.player_cell.is_none());
    assert_eq!(sim.motion.collider.position, [1., 2., 3., 1.]);
    assert_eq!(e.game.inventory.amounts[0], 20);
    assert_eq!(e.game.inventory.amounts[4], 5);
    e.switches[1] = 3;
    assert_eq!(
        sim.select_root(&mut e, [0, 0]).unwrap(),
        root::Selection {
            index: 1,
            quit: true,
            restart: false
        }
    );
    assert_eq!(e.switches[1], 6);
}

#[test]
fn authored_set_painter_follows_first_inputs_and_retains_world_for_menu_mesh() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    assert_eq!(
        root::painter_scene(&d, d.roots[0].unwrap()).unwrap(),
        Some(468)
    );
    assert_eq!(root::painter_scene(&d, d.roots[1].unwrap()).unwrap(), None);
    assert_eq!(
        root::painter_scene(&d, d.roots[2].unwrap()).unwrap(),
        Some(5171)
    );
    assert!(root::painter_scene(&d, d.operators.len()).is_err());
}
