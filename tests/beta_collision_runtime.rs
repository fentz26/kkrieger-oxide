use kkrieger_oxide::game::beta_monster::{
    body::Body,
    collision::Completion,
    hit::{HitRegistry, Shot},
    runtime::CollisionRuntime,
    subtractive::{CandidateWorld, DynamicCell, StaticCell},
    world_cast::AdditiveCell,
    zones::Zone,
};
#[test]
fn adapter_casts_updates_zones_and_preserves_history_on_clear_move() {
    let planes = [
        [1., 0., 0., 2.],
        [-1., 0., 0., 2.],
        [0., 1., 0., 2.],
        [0., -1., 0., 2.],
        [0., 0., 1., 2.],
        [0., 0., -1., 2.],
    ];
    let zone_planes = [
        [1., 0., 0., 0.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let adds = [AdditiveCell {
        planes: &planes,
        neighbors: &[],
        only_for_shot: false,
    }];
    let statics: [StaticCell; 0] = [];
    let lists: [&[usize]; 1] = [&[]];
    let zone_lists: [&[usize]; 1] = [&[0]];
    let mut zones = [Zone {
        planes: &zone_planes,
        enter: 0,
        leave: 0,
        collided: 0,
    }];
    let mut hits = HitRegistry::default();
    hits.shots.insert(
        7,
        Shot {
            kind: 3,
            event: None,
        },
    );
    let mut dynamic = [DynamicCell {
        planes: &planes,
        mode: 1,
        only_for_shot: false,
        monster: false,
        player: false,
        part_box: None,
        collided: 0,
    }];
    let mut runtime = CollisionRuntime {
        adds: &adds,
        candidates: CandidateWorld {
            static_cells: &statics,
            lists: &lists,
            dynamic_cells: &mut dynamic,
        },
        zones: &mut zones,
        zone_lists: &zone_lists,
        hits: &mut hits,
        particle_id: 7,
        skip_cell: Some(0),
        monster_mode: false,
        interrupted: false,
        error: None,
    };
    let mut body = Body::new([-0.5, 0., 0., 1.], Some(0), 3);
    body.particles[0].vectors[0] = [0.5, 0., 0., 1.];
    let old = body.particles[0].vectors[2];
    assert_eq!(
        runtime.collide(&mut body.particles[0], false).unwrap(),
        Completion::Clear
    );
    assert_eq!(body.particles[0].vectors[1], [0.5, 0., 0., 1.]);
    assert_eq!(body.particles[0].vectors[2], old);
    assert_eq!(runtime.zones[0].enter, 8);
    assert_eq!(runtime.zones[0].collided, 8);
    assert!(runtime.error.is_none());
    assert_eq!(
        runtime.candidates.dynamic_cells[0].mode, 1,
        "owned cell restored after response"
    );
    assert_eq!(
        runtime.candidates.dynamic_cells[0].collided, 0,
        "owned cell was excluded"
    );
    let before = body.particles[0].clone();
    runtime.skip_cell = Some(99);
    assert_eq!(
        runtime.collide(&mut body.particles[0], false),
        Err("skip cell out of range")
    );
    assert_eq!(body.particles[0], before);
    runtime.skip_cell = Some(0);
    runtime.hits.shots.get_mut(&7).unwrap().kind = 8;
    assert_eq!(
        runtime.collide(&mut body.particles[0], false),
        Err("particle kind differs from registry")
    );
    assert_eq!(body.particles[0], before);

    let mut event = [0; 45];
    event[2] = 1;
    runtime.hits.shots.get_mut(&7).unwrap().event = Some(event);
    runtime.hits.operators[0][0] = 123;
    runtime.hits.time = 100;
    body.particles[0].flags[3] = 8;
    body.particles[0].vectors[1] = [-0.5, 0., 0., 1.];
    body.particles[0].vectors[0] = [3., 0., 0., 1.];
    assert_eq!(
        runtime.collide(&mut body.particles[0], false).unwrap(),
        Completion::Contact
    );
    assert!(body.particles[0].vectors[0][0] < 2.);
    assert_eq!(runtime.hits.pending.len(), 1);
    assert_eq!(runtime.hits.pending[0][0], 123);
    assert_eq!(runtime.hits.shots[&7].event.unwrap()[12], 1);
    assert_eq!(
        runtime.zones[0].enter, 8,
        "stopped shot suppresses zone masks"
    );
    assert_eq!(runtime.candidates.dynamic_cells[0].mode, 1);

    runtime.hits.shots.insert(
        7,
        Shot {
            kind: 3,
            event: None,
        },
    );
    body = Body::new([-0.5, 0., 0., 1.], None, 3);
    let settings = kkrieger_oxide::game::beta_monster::physics::Settings {
        gravity: 0.02,
        gravity_player: 0.01,
        damping_player_air: 0.,
        damping_player_ground: 0.,
        fly_mode: 0,
        on_ground: false,
    };
    runtime.step_particle(&mut body, 0, settings).unwrap();
    assert_eq!(
        body.particles[0].cell,
        Some(0),
        "reacquired containing hull"
    );
    assert_eq!(body.particles[0].flags[5], 0);
    assert!(body.particles[0].vectors[0][1] < -0.0099 && body.particles[0].vectors[0][1] > -0.0101);
    assert_eq!(body.particles[0].vectors[2][1].to_bits(), 0);
    assert_eq!(body.particles[0].vectors[0][1].to_bits(), 0xbc23d70a);
    let before = body.clone();
    runtime.particle_id = 99;
    assert_eq!(
        runtime.step_particle(&mut body, 0, settings),
        Err("shot identity missing")
    );
    assert_eq!(body, before, "registry errors fail before physics mutation");
}
