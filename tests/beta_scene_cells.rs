use kkrieger_oxide::{
    collision::{Cell, DynamicCell},
    game::beta_monster::{hull::Hull, scene_cells::SceneCell},
    mesh::identity,
};
use std::{cell::RefCell, rc::Rc};
#[test]
fn scene_cell_capture_preserves_identity_and_publishes_released_pose() {
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [2., 4., 6.], 1),
        identity(),
    )));
    let mut captured = SceneCell::capture(Rc::clone(&source), false).unwrap();
    let hull = Hull::from_vertices(source.borrow().original_vertices, 1);
    assert_eq!(
        captured.cell.original[..captured.cell.count],
        hull.planes[..hull.count]
    );
    captured.cell.current[3][0] = 1.;
    captured
        .cell
        .reconcile(std::iter::empty(), |_| unreachable!())
        .unwrap();
    captured.interactions.collided = 0x100;
    captured.publish().unwrap();
    assert_eq!(source.borrow().cell.interactions.collided, 0x100);
    assert_eq!(source.borrow().confirmed_matrix[3][0], 1.);
    assert_eq!(source.borrow().current_matrix, captured.cell.pose.matrix);
    assert_eq!(
        source.borrow().cell.planes,
        captured.cell.pose.planes[..captured.cell.pose.count]
    );
    assert_eq!(source.borrow().blocked, 0);
    assert_eq!(source.borrow().cell.mode & 0x20, 0);
    let retained = Rc::clone(&source);
    let borrowed = retained.borrow_mut();
    assert!(SceneCell::capture(Rc::clone(&source), false).is_err());
    assert!(captured.publish().is_err());
    drop(borrowed);
    captured.cell.pose.blocked = true;
    captured.publish().unwrap();
    assert_eq!(source.borrow().blocked, 1);
}
#[test]
fn player_scene_cell_uses_released_solid_mode_and_shared_actor_tag() {
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [1.; 3], 2),
        identity(),
    )));
    let player = SceneCell::capture(Rc::clone(&source), true).unwrap();
    assert_eq!(player.cell.mode & 3, 1);
    assert!(player.player);
    assert!(player.collision_view().unwrap().player);
    source.borrow_mut().monster = Some(7);
    source.borrow_mut().cell.only_for_shot = true;
    let actor = SceneCell::capture(Rc::clone(&source), false).unwrap();
    assert!(actor.cell.monster);
    assert_eq!(actor.actor, Some(7));
    assert!(actor.collision_view().unwrap().monster);
    assert!(actor.collision_view().unwrap().only_for_shot);
    assert_eq!(actor.collision_view().unwrap().part_box, None);
}

#[test]
fn released_ground_query_includes_ordinary_scene_hulls_and_excludes_tagged_bodies() {
    use kkrieger_oxide::game::beta_monster::{
        HitBox, frame::ActorSimulation, static_world::StaticWorld,
    };
    let mut simulation = ActorSimulation::new(StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]));
    let mut matrix = identity();
    matrix[3][1] = -1.;
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [2., 1., 2.], 1),
        matrix,
    )));
    source.borrow_mut().cell.mode &= !0x20;
    let ordinary = SceneCell::capture(Rc::clone(&source), false).unwrap();
    let clearance = simulation
        .player_clearance_with_cells([0., 0., 0., 1.], Some(0), 0.01, 1., &[ordinary])
        .unwrap();
    assert!((clearance - 0.49).abs() < 0.00001, "clearance {clearance}");
    let player = SceneCell::capture(Rc::clone(&source), true).unwrap();
    let clearance = simulation
        .player_clearance_with_cells([0., 0., 0., 1.], Some(0), 0.01, 1., &[player])
        .unwrap();
    assert!(clearance > 1.);
    source.borrow_mut().monster = Some(7);
    let actor = SceneCell::capture(Rc::clone(&source), false).unwrap();
    let clearance = simulation
        .player_clearance_with_cells([0., 0., 0., 1.], Some(0), 0.01, 1., &[actor])
        .unwrap();
    assert!(clearance > 1.);
}
