use kkrieger_oxide::{
    mesh::identity,
    scene::particles::{Appearance, DrawParameters, Particle, System},
};

#[test]
fn original_material_gate_prevents_geometry_and_aging() {
    let parameters = DrawParameters {
        appearance: Appearance {
            colors: [0xffffffff; 3],
            sizes: [1.; 3],
            middle: 0.5,
            fade: 0.1,
        },
        gravity: 0.,
        flags: 0,
    };
    let mut system = System {
        capacity: 1,
        particles: vec![Particle {
            position: [0., 0., 0., 0.],
            speed: [1., 0., 0., 0.],
            time: 0,
            max_time: 100,
        }],
    };
    for passes in [None, Some(0)] {
        assert!(
            system
                .draw(&parameters, 10, identity(), identity(), passes)
                .unwrap()
                .is_empty()
        );
        assert_eq!(system.particles[0].time, 0);
    }
    let first = system
        .draw(&parameters, 10, identity(), identity(), Some(1))
        .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0][0][0], 1.);
    assert_eq!(system.particles[0].position[0], 10.);
    assert_eq!(system.particles[0].time, 10);
    let second = system
        .draw(&parameters, 10, identity(), identity(), Some(3))
        .unwrap();
    assert_eq!(second.len(), 1); // Original PartSystem uses only the first pass.
    assert_eq!(second[0][0][0], 11.);
    assert_eq!(system.particles[0].time, 20);
}

#[test]
fn original_flag_selects_operator_axes_instead_of_modelview() {
    let mut parameters = DrawParameters {
        appearance: Appearance {
            colors: [0xffffffff; 3],
            sizes: [1.; 3],
            middle: 0.5,
            fade: 0.1,
        },
        gravity: 0.,
        flags: 0,
    };
    let mut system = System {
        capacity: 1,
        particles: vec![Particle {
            position: [0.; 4],
            speed: [0.; 4],
            time: 50,
            max_time: 100,
        }],
    };
    let operator = [
        [0., 1., 0., 0.],
        [-1., 0., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let camera = system
        .draw(&parameters, 0, operator, identity(), Some(1))
        .unwrap();
    assert_eq!(&camera[0][0][..3], &[1., 0., 0.]);
    parameters.flags = 1;
    let local = system
        .draw(&parameters, 0, operator, identity(), Some(1))
        .unwrap();
    assert_eq!(&local[0][0][..3], &[0., -1., 0.]);
}
