use kkrieger_oxide::collision::Intersection;
use kkrieger_oxide::game::beta_monster::{
    hit::{HitRegistry, Shot},
    subtractive::{CandidateQuery, DynamicCell, ParticleInfo, scan_dynamic},
    world_cast::Ray,
};
#[test]
fn registry_queues_effects_and_updates_scan_selection() {
    let planes = [
        [1., 0., 0., 1.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let mut registry = HitRegistry::default();
    let mut words = [0; 45];
    words[2] = 1;
    registry.shots.insert(
        7,
        Shot {
            kind: 8,
            event: Some(words),
        },
    );
    registry.operators[0][0] = 123;
    registry.operators[1][0] = 456;
    registry.time = 100;
    let mut cells = [DynamicCell {
        planes: &planes,
        mode: 1,
        only_for_shot: false,
        monster: false,
        player: false,
        part_box: None,
        collided: 0,
    }];
    let mut q = CandidateQuery {
        ray: Ray {
            direction: [1., 0., 0.],
            start: [-3., 0., 0., 1.],
            end: [0., 0., 0., 1.],
        },
        particle: Some(ParticleInfo { kind: 8, mass: 1. }),
        first: Some(registry.first_particle(7).unwrap()),
        monster_mode: false,
    };
    let mut best = Intersection {
        plane: [0.; 4],
        distance: 0.0001,
    };
    assert!(scan_dynamic(
        &mut cells,
        &mut q,
        &mut best,
        &mut false,
        false,
        &mut registry
    ));
    assert_eq!(q.first.unwrap().shot_select, Some(1));
    assert_eq!(registry.shots[&7].event.unwrap()[5], 100);
    assert_eq!(registry.pending.len(), 1);
    assert_eq!(registry.pending[0][0], 123);
    assert_eq!(cells[0].collided, 1 << 8);
    assert!(registry.error.is_none());
    // Reflection leaves Select=0; a later monster contact chooses its own slot.
    registry.flags[8] = 2;
    registry.shots.get_mut(&7).unwrap().event = Some(words);
    registry.pending.clear();
    q.first = Some(registry.first_particle(7).unwrap());
    best.distance = 0.0001;
    scan_dynamic(
        &mut cells,
        &mut q,
        &mut best,
        &mut false,
        false,
        &mut registry,
    );
    assert_eq!(q.first.unwrap().shot_select, Some(0));
    cells[0].monster = true;
    best.distance = 0.0001;
    scan_dynamic(
        &mut cells,
        &mut q,
        &mut best,
        &mut false,
        false,
        &mut registry,
    );
    assert_eq!(q.first.unwrap().shot_select, Some(1));
    assert_eq!(
        registry.pending.iter().map(|e| e[0]).collect::<Vec<_>>(),
        [123, 456]
    );
}
#[test]
fn missing_identity_is_reported_without_event_insertion() {
    let registry = HitRegistry::default();
    assert!(registry.first_particle(3).is_err());
}
#[test]
fn stale_registry_inputs_preserve_events_and_report_first_error() {
    use kkrieger_oxide::game::beta_monster::subtractive::{DynamicEffects, ParticleContact};
    let mut registry = HitRegistry::default();
    registry.shots.insert(
        7,
        Shot {
            kind: 8,
            event: Some([0; 45]),
        },
    );
    let contact = ParticleContact {
        cell: 0,
        particle: 7,
        kind: 8,
        position: [0.; 4],
        plane: [1., 0., 0., 0.],
        monster: false,
    };
    let mut selected = Some(1);
    registry.particle_hit(contact, &mut selected);
    assert_eq!(registry.error, Some("shot selection differs from registry"));
    assert_eq!(registry.shots[&7].event, Some([0; 45]));
    assert!(registry.pending.is_empty());
    registry.particle_hit(
        ParticleContact {
            particle: 99,
            ..contact
        },
        &mut selected,
    );
    assert_eq!(registry.error, Some("shot selection differs from registry"));
    registry.error = None;
    registry.particle_hit(ParticleContact { kind: 9, ..contact }, &mut selected);
    assert_eq!(registry.error, Some("shot kind differs from registry"));
}
