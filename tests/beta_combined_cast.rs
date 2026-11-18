use kkrieger_oxide::{
    collision::Intersection,
    game::beta_monster::{
        subtractive::{
            CandidateQuery, CandidateWorld, DynamicEffects, ParticleContact, ParticleInfo,
            StaticCell,
        },
        world_cast::{AdditiveCell, Ray, cast_world},
    },
};
fn planes(x: f32, extent: f32) -> [[f32; 4]; 6] {
    [
        [1., 0., 0., extent - x],
        [-1., 0., 0., extent + x],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ]
}
struct Effects;
impl DynamicEffects for Effects {
    fn particle_hit(&mut self, _: ParticleContact, _: &mut Option<i32>) {
        panic!("unexpected dynamic hit")
    }
}
#[test]
fn combined_world_cast_crosses_portal_and_selects_static_obstacle() {
    let a = planes(0., 1.);
    let b = planes(2., 1.);
    let obstacle = planes(1.5, 0.25);
    let adds = [
        AdditiveCell {
            planes: &a,
            neighbors: &[1],
            only_for_shot: false,
        },
        AdditiveCell {
            planes: &b,
            neighbors: &[0],
            only_for_shot: false,
        },
    ];
    let static_cells = [StaticCell {
        planes: &obstacle,
        mode: 1,
        only_for_shot: false,
    }];
    // The released visited sequence contains cells being left, so the first
    // list must contain the obstacle spanning the portal.
    let lists: [&[usize]; 2] = [&[0], &[0]];
    let mut world = CandidateWorld {
        static_cells: &static_cells,
        lists: &lists,
        dynamic_cells: &mut [],
    };
    let mut query = CandidateQuery {
        ray: Ray {
            direction: [1., 0., 0.],
            start: [0., 0., 0., 1.],
            end: [2., 0., 0., 1.],
        },
        particle: Some(ParticleInfo { kind: 3, mass: 1. }),
        first: None,
        monster_mode: false,
    };
    let mut current = 0;
    let mut interrupted = false;
    let hit = cast_world(
        &adds,
        &mut current,
        &mut world,
        &mut query,
        &mut interrupted,
        &mut Effects,
    )
    .unwrap()
    .unwrap();
    assert_eq!(current, 1);
    assert!(!interrupted);
    assert_eq!(
        hit,
        Intersection {
            plane: [-1., -0., -0., 1.25],
            distance: -0.75
        }
    );
    query.ray.start = [1.5, 0., 0., 1.];
    let hit = cast_world(
        &adds,
        &mut current,
        &mut world,
        &mut query,
        &mut interrupted,
        &mut Effects,
    )
    .unwrap()
    .unwrap();
    assert!(interrupted);
    assert_eq!(
        hit,
        Intersection {
            plane: [1., 0., 0., 0.],
            distance: 0.
        }
    );
}
#[test]
fn combined_cast_propagates_invalid_candidate_links() {
    let p = planes(0., 1.);
    let adds = [AdditiveCell {
        planes: &p,
        neighbors: &[],
        only_for_shot: false,
    }];
    let mut world = CandidateWorld {
        static_cells: &[],
        lists: &[&[0]],
        dynamic_cells: &mut [],
    };
    let mut query = CandidateQuery {
        ray: Ray {
            direction: [1., 0., 0.],
            start: [0., 0., 0., 1.],
            end: [0.5, 0., 0., 1.],
        },
        particle: None,
        first: None,
        monster_mode: false,
    };
    assert_eq!(
        cast_world(
            &adds,
            &mut 0,
            &mut world,
            &mut query,
            &mut false,
            &mut Effects
        ),
        Err("static subtractive cell out of range")
    );
}
