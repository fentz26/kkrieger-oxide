use kkrieger_oxide::scene::particles::{Particle, System};

#[test]
fn drawing_precedes_aging_and_swap_removal_visits_each_particle() {
    let mut system = System {
        capacity: 3,
        particles: (0..3)
            .map(|id| Particle {
                position: [id as f32, 10., 0., 1.],
                speed: [1., 2., 3., 4.],
                time: 0,
                max_time: if id == 1 { 10 } else { 0 },
            })
            .collect(),
    };
    let mut drawn = Vec::new();
    system.draw_and_advance(10, 31623., |particle| {
        drawn.push((particle.position, particle.time));
    });
    assert_eq!(
        drawn.iter().map(|p| p.0[0]).collect::<Vec<_>>(),
        [0., 2., 1.]
    );
    assert!(drawn.iter().all(|p| p.1 == 0));
    assert_eq!(system.particles.len(), 1);
    let survivor = &system.particles[0];
    assert_eq!(survivor.time, 10); // Equality survives this draw.
    assert_eq!(survivor.position, [11., -25., 30., 41.]);
    assert_eq!(survivor.speed, [1., -8., 3., 4.]);
    system.draw_and_advance(1, 0., |_| {});
    assert!(system.particles.is_empty());
}
