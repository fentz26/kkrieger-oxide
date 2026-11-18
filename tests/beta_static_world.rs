use kkrieger_oxide::{
    game::beta_monster::{
        HitBox,
        static_world::StaticWorld,
        subtractive::{CandidateQuery, CandidateWorld},
        world_cast::{Ray, cast_world},
    },
    mesh::identity,
};
fn cube(x: f32, scale: f32) -> [[f32; 4]; 8] {
    let mut matrix = identity();
    matrix[3][0] = x;
    HitBox {
        matrix,
        scale: [scale; 3],
    }
    .vertices()
}
#[test]
fn owned_world_maps_interleaved_source_cells_to_additive_registry() {
    let world = StaticWorld::new([
        (cube(0., 0.25), 1),
        (cube(0., 4.), 0),
        (cube(0., 0.5), 2),
        (cube(3., 4.), 0),
    ]);
    assert_eq!(world.adds, [1, 3]);
    assert_eq!(world.neighbors, [vec![1], vec![0]]);
    assert_eq!(world.subtractive_lists, [vec![0], vec![]]);
    assert_eq!(world.zone_lists, [vec![2], vec![]]);
    let adds = world.additive_views();
    let statics = world.static_views();
    let lists: Vec<_> = world.subtractive_lists.iter().map(Vec::as_slice).collect();
    let mut candidates = CandidateWorld {
        static_cells: &statics,
        lists: &lists,
        dynamic_cells: &mut [],
    };
    let mut query = CandidateQuery {
        ray: Ray {
            direction: [1., 0., 0.],
            start: [-1., 0., 0., 1.],
            end: [0., 0., 0., 1.],
        },
        particle: None,
        first: None,
        monster_mode: false,
    };
    let mut current = 0;
    let mut interrupted = false;
    let hit = cast_world(
        &adds,
        &mut current,
        &mut candidates,
        &mut query,
        &mut interrupted,
        &mut kkrieger_oxide::game::beta_monster::hit::HitRegistry::default(),
    )
    .unwrap();
    assert!(hit.is_some());
}
