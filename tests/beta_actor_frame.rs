use kkrieger_oxide::{
    game::{
        Inventory,
        beta_monster::{
            HitBox, ai,
            frame::{ActorSimulation, Settings},
            physics,
            registry::Registry,
            static_world::StaticWorld,
        },
    },
    mesh::identity,
};
#[test]
fn actor_frame_runs_physics_then_ai_with_captured_scene_colliders() {
    let world = StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]);
    let mut simulation = ActorSimulation::new(world);
    let mut registry = Registry::default();
    let mut data = [0; 26];
    data[6] = 1f32.to_bits();
    data[7] = 10f32.to_bits();
    data[8] = 10f32.to_bits();
    data[10] = 20;
    data[12] = 10;
    data[13] = 0.01f32.to_bits();
    data[15] = 0.01f32.to_bits();
    data[20..23].fill(1f32.to_bits());
    let token = registry.spawn(&data, identity(), 0, |_| Some(0)).unwrap();
    registry.register(token).unwrap();
    let mut inventory = Inventory::default();
    inventory.amounts[0] = 100;
    let settings = Settings {
        physics: physics::Settings {
            gravity: 0.,
            gravity_player: 0.,
            damping_player_air: 0.,
            damping_player_ground: 0.,
            fly_mode: 0,
            on_ground: false,
        },
        ai: ai::Settings {
            gravity: 0.,
            height: 1.,
            height_gain: 0.,
        },
        player: [-1., 0., 0., 1.],
        player_cell: Some(0),
        first_particle: 100,
    };
    let sounds = simulation
        .step(&registry, &[], &mut inventory, &settings)
        .unwrap();
    assert_eq!(sounds.len(), 1);
    assert_eq!(inventory.amounts[0], 90);
    let actor = registry.get(token).unwrap();
    assert!(actor.borrow().active);
    assert_ne!(actor.borrow().hit_cell.mode & 0x20, 0);
    let sounds = simulation
        .step(&registry, &[token], &mut inventory, &settings)
        .unwrap();
    assert_eq!(sounds.len(), 1);
    assert_eq!(inventory.amounts[0], 80);
    assert_eq!(actor.borrow().hit_cell.mode & 0x20, 0);
    assert_eq!(simulation.hits.shots.len(), 5);
    assert!(
        actor
            .borrow()
            .body
            .particles
            .iter()
            .all(|particle| particle.cell == Some(0))
    );
    actor.borrow_mut().initial.life = 0;
    assert!(
        simulation
            .step(&registry, &[], &mut inventory, &settings)
            .unwrap()
            .is_empty()
    );
    assert_eq!(inventory.amounts[0], 80);
}
