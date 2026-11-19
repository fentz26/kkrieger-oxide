use kkrieger_oxide::{
    collision::{Cell, DynamicCell},
    game::beta_monster::{
        HitBox, ai,
        frame::{ActorSimulation, FrameInput, Settings},
        physics,
        projectile::{PhysicsSlot, Point, sync_hit_events},
        registry::Registry,
        runtime::RegisteredPoint,
        scene_cells::SceneCell,
        static_world::StaticWorld,
    },
    mesh::identity,
    scene::event::{Event, Scheduler},
};
use std::{cell::RefCell, rc::Rc};
#[test]
fn point_frame_hits_scene_wall_stops_shared_event_and_emits_effect() {
    run_wall_impact(false);
}
#[test]
fn reflected_point_frame_preserves_event_lifetime() {
    run_wall_impact(true);
}
fn run_wall_impact(reflect: bool) {
    let mut simulation = ActorSimulation::new(StaticWorld::new([(
        HitBox {
            matrix: identity(),
            scale: [20.; 3],
        }
        .vertices(),
        0,
    )]));
    let registry = Registry::default();
    let mut events = Scheduler::default();
    let event = events.insert(
        Event {
            start: 50,
            end: 100,
            operator: Some(17),
            ..Default::default()
        },
        true,
    );
    let mut words = [0; 12];
    words[0] = 0x20;
    words[1] = 2.5f32.to_bits();
    words[10] = 1f32.to_bits();
    words[11] = 8;
    let point = Rc::new(RefCell::new(
        Point::new(&words, identity(), Some(0), Some(event)).unwrap(),
    ));
    point
        .borrow()
        .register_hit_state(999, &events, &mut simulation.hits)
        .unwrap();
    simulation.hits.operators[0][0] = 19;
    simulation.hits.time = 20;
    simulation.hits.flags[8] = if reflect { 2 } else { 0 };
    let points = [RegisteredPoint {
        point: Rc::clone(&point),
        particle: 999,
        skip_cell: None,
    }];
    let mut matrix = identity();
    matrix[3][0] = 1.5;
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [1., 4., 4.], 1),
        matrix,
    )));
    source.borrow_mut().cell.mode &= !0x20;
    let scene = vec![SceneCell::capture(source, false).unwrap()];
    let order = [PhysicsSlot::Point(0)];
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
        player: [0., 0., 0., 1.],
        player_cell: Some(0),
        first_particle: 5,
    };
    if !reflect {
        let other = Rc::new(RefCell::new(point.borrow().clone()));
        let invalid = [
            points[0].clone(),
            RegisteredPoint {
                point: other,
                particle: 999,
                skip_cell: None,
            },
        ];
        let before = point.borrow().particle.vectors[0];
        assert!(matches!(
            simulation.physics_frame(
                FrameInput {
                    registry: &registry,
                    colliders: &[],
                    scene: Vec::new(),
                    dynamic_order: None,
                    points: &invalid,
                    physics_order: None,
                },
                &mut [],
                &settings
            ),
            Err("released particle identity overlaps")
        ));
        assert_eq!(point.borrow().particle.vectors[0], before);
        assert!(simulation.hits.pending.is_empty());
    }
    simulation
        .physics_frame(
            FrameInput {
                registry: &registry,
                colliders: &[],
                scene,
                dynamic_order: None,
                points: &points,
                physics_order: Some(&order),
            },
            &mut [],
            &settings,
        )
        .unwrap();
    assert!(point.borrow().particle.vectors[0][0] < 1.);
    assert_eq!(simulation.hits.pending.len(), 1);
    assert_eq!(simulation.hits.pending[0][0], 19);
    sync_hit_events(&events, &simulation.hits).unwrap();
    let event_id = event;
    let event = events.events[&event_id].borrow();
    assert_eq!(
        (event.start, event.end, event.select, event.shot_time),
        if reflect {
            (50, 100, 0, 20)
        } else {
            (0, 1, 1, 20)
        }
    );
    assert_eq!(event.operator, Some(17));
    drop(event);
    events.events.remove(&event_id);
    simulation
        .physics_frame(
            FrameInput {
                registry: &registry,
                colliders: &[],
                scene: Vec::new(),
                dynamic_order: None,
                points: &[],
                physics_order: None,
            },
            &mut [],
            &settings,
        )
        .unwrap();
    assert!(simulation.hits.shots.is_empty());
    sync_hit_events(&events, &simulation.hits).unwrap();
    assert_eq!(
        simulation.hits.pending.len(),
        1,
        "retiring particles preserves queued effects"
    );
}
