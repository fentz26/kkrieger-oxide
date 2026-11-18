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
