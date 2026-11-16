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

#[test]
fn native_original_billboards_match_all_vertex_words() {
    let mut bytes: &[u8] = include_bytes!("fixtures/particle-billboard.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let value = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        value
    }
    fn real(bytes: &mut &[u8]) -> f32 {
        f32::from_bits(word(bytes))
    }
    assert_eq!(word(&mut bytes), 256);
    for case in 0..256 {
        let a = Appearance {
            colors: std::array::from_fn(|_| word(&mut bytes)),
            sizes: std::array::from_fn(|_| real(&mut bytes)),
            middle: real(&mut bytes),
            fade: real(&mut bytes),
        };
        let p = Particle {
            position: std::array::from_fn(|_| real(&mut bytes)),
            speed: [0.; 4],
            time: word(&mut bytes) as i32,
            max_time: word(&mut bytes) as i32,
        };
        let basis = std::array::from_fn(|_| std::array::from_fn(|_| real(&mut bytes)));
        let vertices = a.billboard(&p, basis).unwrap();
        for (index, value) in vertices.into_iter().flatten().enumerate() {
            assert_eq!(
                value.to_bits(),
                word(&mut bytes),
                "case {case}, word {index}"
            );
        }
    }
    assert!(bytes.is_empty());
}

#[test]
fn billboard_axes_transpose_before_normalizing() {
    let matrix = [
        [0., 3., 7., 0.],
        [4., 0., 8., 0.],
        [0., 4., 9., 0.],
        [100., 200., 300., 1.],
    ];
    assert_eq!(
        kkrieger_oxide::scene::particles::billboard_basis(matrix),
        [[0., 1., 0.], [0.6, 0., 0.8]]
    );
}
