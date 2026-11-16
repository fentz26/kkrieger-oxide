use kkrieger_oxide::{
    collision::world::World,
    game::simulation::Simulation,
    kx::Document,
    random::Random,
    scene::{
        exec::Execution,
        overlay::{Pool, Viewport},
    },
};
#[test]
#[ignore = "generates the full authored collision world"]
fn authored_gameplay_root_executes_its_first_cpu_frame() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d.roots[2].unwrap();
    let mut e = Execution::new_player(&d);
    e.prepare_game_operators(&d, root).unwrap();
    let position = e.game.parameters.start_position;
    assert_eq!(position, [0., 1.75, -21., 1.]);
    let world = World::from_document(&d, 5171).unwrap();
    let mut sim = Simulation::new(world, &mut e, position, 0, [0, 0], Random::new(1)).unwrap();
    sim.restart(&mut e, [0, 0]).unwrap();
    e.overlay = Some(Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    ));
    sim.prepare_scene(&mut e, 0, 0, 1., false).unwrap();
    sim.execute_scene(&mut e, &d, root).unwrap();
    assert_eq!(e.game.monsters.len(), 132);
    assert_eq!(e.render_passes.len(), 2);
    assert!(sim.player_cell.is_some());
    assert_eq!(sim.motion.collider.position, position);
}
