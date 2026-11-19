use kkrieger_oxide::{game::beta_monster::registry::Registry, mesh::identity};
#[test]
fn released_actor_identity_survives_frames_and_is_never_reused() {
    let mut registry = Registry::default();
    let data = [0; 26];
    assert!(registry.spawn(&[], identity(), 0, |_| None).is_err());
    let first = registry.spawn(&data, identity(), 7, |_| Some(3)).unwrap();
    assert_eq!(first, 1);
    let second = registry.spawn(&data, identity(), 8, |_| Some(3)).unwrap();
    registry.register(second).unwrap();
    registry.register(first).unwrap();
    registry.register(second).unwrap();
    assert_eq!(registry.active_ids(), [second, first, second]);
    let retained = registry.get(first).unwrap();
    retained.borrow_mut().initial.life = 17;
    assert_eq!(retained.borrow().initial.event_words[43], first);
    registry.begin_frame();
    assert!(registry.active_ids().is_empty());
    assert_eq!(registry.get(first).unwrap().borrow().initial.life, 17);
    registry.register(first).unwrap();
    registry.remove(first).unwrap();
    assert!(registry.active_ids().is_empty());
    assert!(registry.register(first).is_err());
    let third = registry.spawn(&data, identity(), 9, |_| None).unwrap();
    assert!(third > second);
    assert!(registry.get(first).is_none());
    assert_eq!(retained.borrow().initial.life, 17);
}

#[test]
fn ordered_registry_visits_mutate_shared_actor_storage() {
    let mut registry = Registry::default();
    let first = registry.spawn(&[0; 26], identity(), 0, |_| None).unwrap();
    let second = registry.spawn(&[0; 26], identity(), 0, |_| None).unwrap();
    registry.register(second).unwrap();
    registry.register(first).unwrap();
    registry.register(second).unwrap();
    let mut order = Vec::new();
    registry
        .visit_active(|token, actor| {
            order.push(token);
            actor.initial.life += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(order, [second, first, second]);
    assert_eq!(registry.get(first).unwrap().borrow().initial.life, 1);
    assert_eq!(registry.get(second).unwrap().borrow().initial.life, 2);
    let retained = registry.get(second).unwrap();
    let borrowed = retained.borrow_mut();
    assert_eq!(
        registry.visit_active(|_, _| Ok(())),
        Err("released actor already borrowed")
    );
    drop(borrowed);
    registry.visit_active(|_, _| Ok(())).unwrap();
}

#[test]
fn scene_collider_capture_keeps_particle_ids_stable_and_actor_borrows_free() {
    use kkrieger_oxide::game::beta_monster::registration::ActorFrame;
    let mut registry = Registry::default();
    let data = [0; 26];
    let first = registry.spawn(&data, identity(), 0, |_| Some(0)).unwrap();
    let second = registry.spawn(&data, identity(), 0, |_| Some(0)).unwrap();
    registry.register(second).unwrap();
    registry.register(first).unwrap();
    let mut frame = ActorFrame::capture(&registry, &[second], 100).unwrap();
    assert_eq!(frame.bodies[&first].particles, [100, 101, 102, 103, 104]);
    assert_eq!(frame.bodies[&second].particles, [105, 106, 107, 108, 109]);
    assert_eq!(frame.bodies[&first].skip_cell, None);
    assert_eq!(frame.bodies[&second].skip_cell, Some(0));
    let views = frame.dynamic_views().unwrap();
    assert_eq!(views.len(), 1);
    registry
        .visit_active(|_, actor| {
            actor.initial.life = 9;
            Ok(())
        })
        .unwrap();
    assert_eq!(registry.get(second).unwrap().borrow().initial.life, 9);
    assert_eq!(views[0].mode, frame.cells[0].1.mode);
    assert_eq!(frame.particles.len(), 10);
    drop(views);
    assert!(frame.save_collision_masks(&[]).is_err());
    frame.save_collision_masks(&[0x80]).unwrap();
    assert_eq!(frame.dynamic_views().unwrap()[0].collided, 0x80);
    frame.cells[0].1.mode = 1;
    frame.publish_geometry(&registry).unwrap();
    assert_eq!(registry.get(second).unwrap().borrow().hit_cell.mode, 1);
    assert_eq!(registry.get(second).unwrap().borrow().initial.life, 9);

    assert!(ActorFrame::capture(&registry, &[99], 100).is_err());
    assert!(ActorFrame::capture(&registry, &[], usize::MAX).is_err());
}

#[test]
fn collision_weapon_bits_damage_shared_actors_once_in_weapon_order() {
    use kkrieger_oxide::game::beta_monster::registration::ActorFrame;
    let mut registry = Registry::default();
    let mut data = [0; 26];
    data[10] = 100;
    data[11] = 20;
    data[17] = 9;
    let token = registry.spawn(&data, identity(), 0, |_| Some(0)).unwrap();
    registry.register(token).unwrap();
    let mut frame = ActorFrame::capture(&registry, &[token, token], 5).unwrap();
    frame.save_collision_masks(&[0x101, 0x300]).unwrap();
    let mut switches = [0; 256];
    let damage = [30, 90, 0, 0, 0, 0, 0, 0];
    let actor = registry.get(token).unwrap();
    let borrowed = actor.borrow_mut();
    assert_eq!(
        frame.apply_weapon_hits(&registry, &damage, &mut switches, |_| Some(0)),
        Err("released actor already borrowed")
    );
    assert_eq!(frame.dynamic_views().unwrap()[0].collided, 0x101);
    assert_eq!(borrowed.initial.life, 100);
    drop(borrowed);
    frame
        .apply_weapon_hits(&registry, &damage, &mut switches, |_| Some(0))
        .unwrap();
    assert_eq!(registry.get(token).unwrap().borrow().initial.life, 10);
    assert!(registry.get(token).unwrap().borrow().active);
    frame
        .apply_weapon_hits(&registry, &damage, &mut switches, |_| Some(0))
        .unwrap();
    assert_eq!(registry.get(token).unwrap().borrow().initial.life, 10);
    frame.save_collision_masks(&[0x8000, 0]).unwrap();
    frame
        .apply_weapon_hits(&registry, &[20; 8], &mut switches, |_| Some(0))
        .unwrap();
    assert_eq!(registry.get(token).unwrap().borrow().initial.life, 5);
    assert_eq!(switches[9], 0);
    frame.save_collision_masks(&[0x300, 0x300]).unwrap();
    frame
        .apply_weapon_hits(&registry, &damage, &mut switches, |_| Some(0))
        .unwrap();
    assert_eq!(registry.get(token).unwrap().borrow().initial.life, 0);
    assert_eq!(switches[9], 1);
}

#[test]
fn mixed_collider_order_remaps_owned_cells_and_collision_masks() {
    use kkrieger_oxide::{
        collision::{Cell, DynamicCell},
        game::beta_monster::{
            registration::{ActorFrame, DynamicSlot},
            scene_cells::SceneCell,
        },
    };
    use std::{cell::RefCell, rc::Rc};
    let mut registry = Registry::default();
    let token = registry
        .spawn(&[0; 26], identity(), 0, |_| Some(0))
        .unwrap();
    registry.register(token).unwrap();
    let source = Rc::new(RefCell::new(DynamicCell::new(
        Cell::new(identity(), [1.; 3], 2),
        identity(),
    )));
    let mut frame = ActorFrame::capture(&registry, &[token], 5).unwrap();
    frame
        .attach_scene(vec![SceneCell::capture(source, true).unwrap()])
        .unwrap();
    frame
        .set_order(&[DynamicSlot::Scene(0), DynamicSlot::Actor(0)])
        .unwrap();
    assert_eq!(frame.bodies[&token].skip_cell, Some(1));
    frame.save_collision_masks(&[0x100, 0x200]).unwrap();
    let views = frame.dynamic_views().unwrap();
    assert!(views[0].player);
    assert_eq!(views[0].collided, 0x100);
    assert_eq!(views[1].part_box, Some(token as usize));
    assert_eq!(views[1].collided, 0x200);
    drop(views);
    assert!(
        frame
            .set_order(&[DynamicSlot::Actor(0), DynamicSlot::Actor(0)])
            .is_err()
    );
    assert_eq!(frame.bodies[&token].skip_cell, Some(1));
}
