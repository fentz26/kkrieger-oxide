use kkrieger_oxide::{
    collision::world::World,
    game::simulation::Simulation,
    kx::Document,
    material,
    random::Random,
    scene::{
        exec::Execution,
        overlay::{Pool, Viewport},
    },
};
#[test]
#[ignore = "generates the full authored collision world"]
fn authored_gameplay_root_executes_movement_and_firing_ticks() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d.roots[2].unwrap();
    let material_roots: Vec<_> = d
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(d.classes[op.class].id, 0xd0 | 0xd1))
        .map(|(i, _)| i)
        .collect();
    let materials: std::collections::BTreeMap<_, _> = material_roots
        .iter()
        .copied()
        .zip(material::generate_many(&d, &material_roots).unwrap())
        .collect();
    let mut particle_packets = 0;
    let mut particle_quads = 0;
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
    for pass in e.render_passes.clone() {
        let packets = e.draw_particle_pass(&d, &pass, &materials).unwrap();
        particle_packets += packets.len();
        particle_quads += packets.iter().map(|p| p.quads.len()).sum::<usize>();
        for quad in packets.iter().flat_map(|p| &p.quads) {
            assert!(
                quad.iter()
                    .all(|v| v[..3].iter().chain(&v[6..]).all(|f| f.is_finite()))
            );
        }
    }
    assert_eq!(e.game.monsters.len(), 132);
    assert_eq!(e.render_passes.len(), 2);
    assert!(sim.player_cell.is_some());
    assert_eq!(sim.motion.collider.position, position);
    // Original refill/unlock inputs allow selecting an ammo-consuming weapon.
    sim.key(&mut e, 109, [0, 0]).unwrap();
    sim.key(&mut e, 110, [0, 0]).unwrap();
    sim.key(&mut e, 50, [0, 0]).unwrap();
    assert_eq!(e.game.weapon.next, 1);
    assert!(e.game.events.shot[1].is_some());
    let ammo = e.game.inventory.amounts;
    e.finish_events();
    sim.key(&mut e, 119, [0, 0]).unwrap();
    sim.key(&mut e, kkrieger_oxide::game::input::MOUSE_LEFT, [0, 0])
        .unwrap();
    for step in 1..=30 {
        let now = step * 10;
        assert!(sim.tick(&mut e, &d, 1, now, [0, 0]).unwrap().ran);
        sim.prepare_scene(&mut e, now * 64, now, 1., false).unwrap();
        sim.execute_scene(&mut e, &d, root).unwrap();
        for pass in e.render_passes.clone() {
            let packets = e.draw_particle_pass(&d, &pass, &materials).unwrap();
            particle_packets += packets.len();
            particle_quads += packets.iter().map(|p| p.quads.len()).sum::<usize>();
            for quad in packets.iter().flat_map(|p| &p.quads) {
                assert!(
                    quad.iter()
                        .all(|v| v[..3].iter().chain(&v[6..]).all(|f| f.is_finite()))
                );
            }
        }
        assert_eq!(e.game.monsters.len(), 132);
        assert_eq!(e.render_passes.len(), 2);
        assert!(sim.player_cell.is_some());
        assert!(sim.motion.collider.position.into_iter().all(f32::is_finite));
        e.finish_events();
    }
    eprintln!("authored particle packets={particle_packets} quads={particle_quads}");
    assert!(particle_packets > 0);
    assert!(particle_quads > 0);
    assert_eq!(sim.tick_count, 30);
    assert!(sim.motion.collider.position[2] > position[2]);

    assert!(
        e.game.inventory.amounts[4..]
            .iter()
            .zip(&ammo[4..])
            .any(|(after, before)| after < before),
        "firing must consume ammunition: current={} next={} timer={} fire={} before={:?} after={:?}",
        e.game.weapon.current,
        e.game.weapon.next,
        e.game.weapon.timer,
        e.game.weapon.fire_key,
        ammo,
        e.game.inventory.amounts
    );
}
