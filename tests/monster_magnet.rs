use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        monster_actor::{Actor, Context},
        monster_magnet,
    },
    mesh::identity,
    random::Random,
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
}
#[test]
fn original_magnet_pair_order_cap_and_random_stream_match() {
    let mut r = Reader(include_bytes!("fixtures/monster-magnet.bin"));
    assert_eq!(r.word(), 64);
    let cell = Cell::new(identity(), [100.; 3], 0);
    let mut world = World {
        connectivity: connectivity::connect(std::slice::from_ref(&cell)),
        entries: vec![Entry {
            cell,
            sector: None,
            events: [None, None],
            logic: Logic {
                code: 0,
                condition: 0,
                value: 0,
                output: 0,
                events: [None, None],
            },
        }],
        adds: vec![0],
        subs: vec![],
        zones: vec![],
        faces: vec![Some(vec![])],
    };
    let mut p = [0; 29];
    p[10] = 1;
    p[20] = 1f32.to_bits();
    p[21] = 1f32.to_bits();
    p[22] = 1f32.to_bits();
    p[26] = 4;
    for case in 0..64 {
        let count = r.word();
        let mut rng = Random::new(r.int());
        let mut actors = Vec::new();
        for id in 0..count {
            let state = r.int();
            let position = r.vector();
            let mut actor = Actor::spawn(
                id as usize,
                &p,
                Context {
                    world: &mut world,
                    dynamic: &[],
                    player_position: [-1., 0., 2., 1.],
                    parent: identity(),
                    spline: None,
                    time: 0,
                    time_reset: false,
                    flags: None,
                },
            )
            .unwrap();
            actor.health.state = state;
            actor.body.position = position;
            actor.magnet_force = [11., 12., 13., 14.];
            actors.push(actor);
        }
        monster_magnet::apply(&mut actors.iter_mut().collect::<Vec<_>>(), &mut rng);
        for (i, actor) in actors.iter().enumerate() {
            for (a, b) in actor.magnet_force.into_iter().zip(r.vector()) {
                assert_eq!(a.to_bits(), b.to_bits(), "{case}/{i}: {a} vs {b}");
            }
        }
        assert_eq!(rng.next_u32(), r.word(), "random stream {case}");
    }
    assert!(r.0.is_empty());
}
