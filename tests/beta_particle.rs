use kkrieger_oxide::game::beta_monster::{
    body::{Body, Particle},
    physics::{Settings, World},
};
type Raw = [u32; 32];
fn cell(token: u32) -> Option<usize> {
    match token {
        0 => None,
        0x12345678 => Some(7),
        0x87654321 => Some(9),
        _ => panic!("unknown cell token"),
    }
}
fn token(cell: Option<usize>) -> u32 {
    match cell {
        None => 0,
        Some(7) => 0x12345678,
        Some(9) => 0x87654321,
        _ => panic!("unknown cell index"),
    }
}
fn decode(raw: Raw) -> Particle {
    Particle {
        flags: raw[..7].try_into().unwrap(),
        cell: cell(raw[7]),
        weight: f32::from_bits(raw[11]),
        damping: f32::from_bits(raw[10]),
        force: std::array::from_fn(|i| f32::from_bits(raw[12 + i])),
        vectors: std::array::from_fn(|v| {
            std::array::from_fn(|i| f32::from_bits(raw[16 + v * 4 + i]))
        }),
    }
}
fn encode(p: &Particle) -> Raw {
    let mut raw = [0; 32];
    raw[..7].copy_from_slice(&p.flags);
    raw[7] = token(p.cell);
    raw[8] = 0x3000348;
    raw[10] = p.damping.to_bits();
    raw[11] = p.weight.to_bits();
    raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
    for (out, value) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
        *out = value.to_bits();
    }
    raw
}
struct Boundary {
    expected_position: [f32; 4],
    outside: bool,
    found: Option<usize>,
    collided: bool,
    response: [f32; 4],
    response_y: f32,
    trace: Vec<u32>,
    input: Option<Raw>,
}
impl World for Boundary {
    fn outside(&mut self, cell: usize, position: [f32; 4]) -> bool {
        assert_eq!(cell, 7);
        assert_eq!(
            position.map(f32::to_bits),
            self.expected_position.map(f32::to_bits)
        );
        self.trace.push(1);
        self.outside
    }
    fn find_cell(&mut self, position: [f32; 4]) -> Option<usize> {
        assert_eq!(
            position.map(f32::to_bits),
            self.expected_position.map(f32::to_bits)
        );
        self.trace.push(2);
        self.found
    }
    fn collide(&mut self, particle: &mut Particle) -> bool {
        self.trace.push(3);
        self.input = Some(encode(particle));
        particle.vectors[0] = self.response;
        particle.vectors[2][1] = self.response_y;
        self.collided
    }
}
#[test]
fn particle_step_matches_released_world_boundary_trace() {
    let bytes = include_bytes!("fixtures/beta-particle.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let input: [Raw; 5] =
            std::array::from_fn(|_| std::array::from_fn(|_| words.next().unwrap()));
        let kind = words.next().unwrap() as i32;
        let index = words.next().unwrap() as usize;
        let mut settings = Settings {
            gravity: f32::from_bits(words.next().unwrap()),
            gravity_player: f32::from_bits(words.next().unwrap()),
            damping_player_air: f32::from_bits(words.next().unwrap()),
            damping_player_ground: f32::from_bits(words.next().unwrap()),
            fly_mode: 0,
            on_ground: false,
        };
        settings.fly_mode = words.next().unwrap() as i32;
        settings.on_ground = words.next().unwrap() != 0;
        let outside = words.next().unwrap() != 0;
        let found = cell(words.next().unwrap());
        let collided = words.next().unwrap() != 0;
        let response = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let response_y = f32::from_bits(words.next().unwrap());
        let mut body = Body::new([0., 0., 0., 1.], None, kind);
        body.particles = input.map(decode);
        let mut world = Boundary {
            expected_position: body.particles[index].vectors[1],
            outside,
            found,
            collided,
            response,
            response_y,
            trace: Vec::new(),
            input: None,
        };
        body.step_particle(index, settings, &mut world).unwrap();
        for (particle, state) in body.particles.iter().enumerate() {
            for (field, value) in encode(state).into_iter().enumerate() {
                assert_eq!(
                    value,
                    words.next().unwrap(),
                    "case {case} particle {particle} field {field}"
                );
            }
        }
        let count = words.next().unwrap() as usize;
        let trace: [u32; 3] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(world.trace, &trace[..count], "case {case}");
        let expected: Raw = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(
            world.input.unwrap_or([0; 32]),
            expected,
            "case {case} before collision"
        );
        body.apply_forces();
        for (particle, state) in body.particles.iter().enumerate() {
            for (field, value) in encode(state).into_iter().enumerate() {
                assert_eq!(
                    value,
                    words.next().unwrap(),
                    "case {case} particle {particle} field {field} after forces"
                );
            }
        }
    }
    assert!(words.next().is_none());
}
#[test]
fn invalid_particle_groups_fail_before_world_queries() {
    let settings = Settings {
        gravity: 0.01,
        gravity_player: 0.02,
        damping_player_air: 0.1,
        damping_player_ground: 0.2,
        fly_mode: 0,
        on_ground: false,
    };
    let mut world = Boundary {
        expected_position: [0.; 4],
        outside: false,
        found: None,
        collided: false,
        response: [0.; 4],
        response_y: 0.,
        trace: Vec::new(),
        input: None,
    };
    let mut body = Body::new([1., 2., 3., 1.], None, 3);
    let before = body.clone();
    assert_eq!(
        body.step_particle(5, settings, &mut world),
        Err("particle index out of range")
    );
    assert_eq!(body, before);
    body.particles[2].flags[3] = 3;
    let before = body.clone();
    assert_eq!(
        body.step_particle(2, settings, &mut world),
        Err("five-particle group truncated")
    );
    assert_eq!(body, before);
    assert!(world.trace.is_empty());
}
