use kkrieger_oxide::{
    game::beta_monster::{
        actor::Actor,
        ai::{Settings, World},
    },
    mesh::identity,
};
#[derive(Default)]
struct TestWorld {
    calls: Vec<&'static str>,
}
impl World for TestWorld {
    fn find_cell(&mut self, _: [f32; 4]) -> Option<usize> {
        self.calls.push("find");
        Some(7)
    }
    fn occluded(&mut self, _: [f32; 4], _: [f32; 4]) -> bool {
        self.calls.push("visibility");
        false
    }
    fn ground_distance(&mut self, _: [f32; 4], cell: Option<usize>, _: f32) -> Option<f32> {
        assert_eq!(cell, Some(7));
        self.calls.push("ground");
        None
    }
}
fn actor() -> Actor {
    let mut data = [0; 26];
    data[6] = 1f32.to_bits();
    data[7] = 10f32.to_bits();
    data[8] = 10f32.to_bits();
    data[10] = 20;
    data[13] = 1f32.to_bits();
    data[15] = 1f32.to_bits();
    let mut actor = Actor::spawn(&data, identity(), 0, 1, |_| None).unwrap();
    actor.body.particles[0].vectors[0] = [5., 0., 0., 1.];
    actor
}
#[test]
fn tick_activates_moves_and_updates_persistent_event() {
    let mut actor = actor();
    let mut world = TestWorld::default();
    let settings = Settings {
        gravity: 0.,
        height: 1.,
        height_gain: 0.,
    };
    assert!(
        actor
            .tick([0., 0., 0., 1.], &settings, &mut world)
            .is_none()
    );
    assert_eq!(world.calls, ["visibility", "find", "ground"]);
    assert!(actor.active);
    assert!(actor.body.particles[0].vectors[0][0] < 5.);
    assert_eq!(
        actor.initial.event_words[21..25],
        actor.body.particles[0].vectors[0].map(f32::to_bits)
    );
    assert_eq!(actor.initial.event_words[15], 1f32.to_bits());
    assert_eq!(actor.initial.event_words[44], 0x23);
    assert!(actor.cooldowns.into_iter().all(|v| v > 0. && v < 1.));
}
#[test]
fn dead_actor_tick_preserves_state_and_skips_world() {
    let mut actor = actor();
    actor.initial.life = 0;
    let before = actor.clone();
    let mut world = TestWorld::default();
    actor.tick(
        [0.; 4],
        &Settings {
            gravity: 0.,
            height: 1.,
            height_gain: 0.,
        },
        &mut world,
    );
    assert!(world.calls.is_empty());
    assert_eq!(actor.body, before.body);
    assert_eq!(actor.initial, before.initial);
    assert_eq!(actor.cooldowns, before.cooldowns);
}
