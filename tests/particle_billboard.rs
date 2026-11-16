use kkrieger_oxide::scene::particles::{Appearance, Particle};

#[test]
fn original_quad_order_size_and_fade_are_evaluated_before_aging() {
    let particle = Particle {
        position: [10., 20., 30., 0.],
        speed: [0.; 4],
        time: 512,
        max_time: 1024,
    };
    let appearance = Appearance {
        colors: [0xffff0000, 0xff00ff00, 0xff0000ff],
        sizes: [1., 2., 3.],
        middle: 0.5,
        fade: 0.1,
    };
    let basis = [[1., 0., 0.], [0., 1., 0.]];
    let vertices = appearance.billboard(&particle, basis).unwrap();
    assert_eq!(
        vertices.map(|v| [v[0], v[1], v[2], v[6], v[7]]),
        [
            [12., 20., 30., 0., 0.],
            [10., 18., 30., 0., 1.],
            [8., 20., 30., 1., 1.],
            [10., 22., 30., 1., 0.],
        ]
    );
    assert!(vertices.iter().all(|v| v[3] == 0. && v[4] == 0.));
    let color = vertices[0][5].to_bits();
    assert_eq!(color, 0xff00ff00);
    let mut newborn = particle.clone();
    newborn.time = 0;
    let birth = appearance.billboard(&newborn, basis).unwrap();
    assert_eq!(birth[0][5].to_bits() & 0x00ffffff, 0);
}

#[test]
fn undefined_authored_lifetime_division_returns_an_error() {
    let p = Particle {
        position: [0.; 4],
        speed: [0.; 4],
        time: 0,
        max_time: 0,
    };
    let a = Appearance {
        colors: [0; 3],
        sizes: [1.; 3],
        middle: 0.5,
        fade: 0.5,
    };
    assert!(a.billboard(&p, [[1., 0., 0.], [0., 1., 0.]]).is_err());
}
