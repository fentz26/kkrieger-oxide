use kkrieger_oxide::{
    kx::{Class, Document},
    scene::exec::Execution,
};
#[test]
fn game_precalc_sets_only_reachable_authored_start_positions_and_particle_slots() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.events.clear();
    d.classes = [0x10, 0xc1, 0x64]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    d.operators.truncate(5);
    for op in &mut d.operators {
        op.class = 0;
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![0; 32];
    }
    for (index, pos) in [(0, [1f32, 2., 3.]), (1, [4., 5., 6.]), (3, [99., 98., 97.])] {
        d.operators[index].parameters[29..32].copy_from_slice(&pos.map(f32::to_bits));
    }
    d.operators[2].class = 1;
    d.operators[2].inputs = vec![0, 1, 4];
    d.operators[4].class = 2;
    d.operators[4].parameters = vec![2, 48];
    let mut e = Execution::new_player(&d);
    let inventory = e.game.inventory.clone();
    e.game.parameters.gravity = 0.123;
    e.prepare_game_operators(&d, 2).unwrap();
    assert_eq!(e.game.parameters.start_position, [4., 5., 6., 1.]);
    assert_eq!(e.game.parameters.gravity, 0.123);
    assert_eq!(e.game.inventory, inventory);
    assert_eq!(e.particles[2].capacity, 48);
    assert!(e.jobs.is_empty());
    e.prepare_game_operators(&d, 0).unwrap();
    assert_eq!(e.game.parameters.start_position, [1., 2., 3., 1.]);
    assert_eq!(e.particles[2].capacity, 48);
}
