use kkrieger_oxide::{
    game::beta_monster::{
        actor::Actor,
        ai::{Settings, World},
    },
    mesh::Matrix,
};
struct NativeWorld {
    present: bool,
    blocked: bool,
}
impl World for NativeWorld {
    fn find_cell(&mut self, _: [f32; 4]) -> Option<usize> {
        self.present.then_some(0x3006000)
    }
    fn occluded(&mut self, _: [f32; 4], _: [f32; 4]) -> bool {
        self.blocked
    }
    fn ground_distance(&mut self, _: [f32; 4], _: Option<usize>, _: f32) -> Option<f32> {
        None
    }
}
#[test]
fn complete_tick_matches_released_execution() {
    let bytes = include_bytes!("fixtures/beta-ai-tick.bin");
    assert_eq!(bytes.len(), 32260);
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(32));
    for case in 0..32 {
        let token = words.next().unwrap();
        let present = words.next().unwrap() != 0;
        let data: [u32; 26] = std::array::from_fn(|_| words.next().unwrap());
        let matrix: Matrix =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let mut world = NativeWorld {
            present,
            blocked: case % 2 != 0,
        };
        let mut actor = Actor::spawn(&data, matrix, 0, token, |p| world.find_cell(p)).unwrap();
        assert!(
            actor
                .tick(
                    [0., 0., 0., 1.],
                    &Settings {
                        gravity: 0.01,
                        height: 1.,
                        height_gain: 0.2
                    },
                    &mut world
                )
                .is_none()
        );
        let body: [u32; 160] = std::array::from_fn(|_| words.next().unwrap());
        for (i, p) in actor.body.particles.iter().enumerate() {
            for (v, vector) in p.vectors.iter().enumerate() {
                assert_eq!(
                    vector.map(f32::to_bits),
                    body[i * 32 + 16 + v * 4..i * 32 + 20 + v * 4],
                    "case {case} particle {i} vector {v}"
                );
            }
            assert_eq!(
                p.cell.map_or(0, |v| v as u32),
                body[i * 32 + 7],
                "case {case} cell"
            );
        }
        let event: [u32; 45] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(actor.initial.event_words, event, "case {case} event");
        assert_eq!(
            actor.cooldowns.map(f32::to_bits),
            [words.next().unwrap(), words.next().unwrap()],
            "case {case} cooldown"
        );
        assert_eq!(
            u32::from(actor.active),
            words.next().unwrap(),
            "case {case} active"
        );
    }
    assert!(words.next().is_none());
}
