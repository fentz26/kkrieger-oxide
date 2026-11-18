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
