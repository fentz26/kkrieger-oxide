use kkrieger_oxide::{
    collision::Intersection,
    game::beta_monster::{
        body::Body,
        dynamic::{CollisionProperties, MovingCell, Pose},
        subtractive::{CandidateQuery, DynamicEffects, ParticleContact, scan_dynamic},
        world_cast::Ray,
    },
};
struct Effects;
impl DynamicEffects for Effects {
    fn particle_hit(&mut self, _: ParticleContact, _: &mut Option<i32>) {
        panic!("unexpected hit callback")
    }
}
#[test]
fn collision_views_publish_accepted_planes_and_retain_blocked_geometry() {
    let original = [
        [1., 0., 0., 1.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let identity = kkrieger_oxide::mesh::identity();
    let mut cell = MovingCell {
        original,
        count: 6,
        confirmed: identity,
        current: identity,
        mode: 1,
        monster: false,
        pose: Pose::apply(&original, identity).unwrap(),
    };
    let properties = CollisionProperties {
        only_for_shot: false,
        player: false,
        part_box: Some(4),
        collided: 8,
    };
    let query = CandidateQuery {
        ray: Ray {
            direction: [1., 0., 0.],
            start: [-3., 0., 0., 1.],
            end: [0., 0., 0., 1.],
        },
        particle: None,
        first: None,
        monster_mode: false,
    };
    let cast = |cell: &MovingCell| {
        let mut view = [cell.collision_view(properties).unwrap()];
        assert_eq!(view[0].part_box, Some(4));
        assert_eq!(view[0].collided, 8);
        let mut best = Intersection {
            plane: [0.; 4],
            distance: 0.0001,
        };
        let mut query = query;
        scan_dynamic(
            &mut view,
            &mut query,
            &mut best,
            &mut false,
            false,
            &mut Effects,
        )
    };
    assert!(cast(&cell));
    cell.current[3][0] = 2.;
    cell.reconcile(std::iter::empty(), |_| panic!("unexpected probe"))
        .unwrap();
    assert!(!cast(&cell), "accepted transform replaces old hull");
    // A no-motion contained particle blocks reconciliation without changing hull.
    let mut body = Body::new([2., 0., 0., 1.], Some(0), 8);
    body.particles[0].vectors[0][3] = 1.;
    cell.reconcile(std::iter::once((&mut body.particles[0], false)), |_| {
        panic!("unexpected probe")
    })
    .unwrap();
    assert!(cell.pose.blocked);
    assert!(!cast(&cell));
    assert_eq!(
        cell.collision_view(properties).unwrap().planes,
        &cell.pose.planes[..6]
    );
}
