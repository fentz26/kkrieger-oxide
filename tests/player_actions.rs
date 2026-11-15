use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{Inventory, Parameters, player::Motion},
    mesh::identity,
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
#[test]
fn original_jump_and_damage_preserve_gating_armor_and_sound_order() {
    let cells = vec![Cell::new(identity(), [10.; 3], 0)];
    let graph = connectivity::connect(&cells);
    let world = World {
        entries: cells
            .into_iter()
            .map(|cell| Entry {
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
            })
            .collect(),
        adds: vec![0],
        subs: vec![],
        zones: vec![],
        connectivity: graph,
        faces: vec![None],
    };
    let mut r = Reader(include_bytes!("fixtures/player-actions.bin"));
    assert_eq!(r.word(), 256);
    assert_eq!(r.word(), 4);
    for case in 0..256 {
        let mut inventory = Inventory::default();
        inventory.amounts[0] = r.word() as i32;
        inventory.amounts[1] = r.word() as i32;
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        for call in 0..4 {
            let hits = r.word() as i32;
            motion.on_ground = r.word() != 0;
            motion.jump_timer = r.word() as i32;
            motion.speed_up = r.float();
            let parameters = Parameters {
                jump_force: r.float(),
                ..Default::default()
            };
            let jump = motion.jump(&parameters);
            let damage = inventory.hit(hits);
            let sounds: Vec<_> = jump.into_iter().chain(damage).collect();
            assert_eq!(inventory.amounts[0], r.word() as i32, "{case}/{call}");
            assert_eq!(inventory.amounts[1], r.word() as i32);
            assert_eq!(motion.jump_timer, r.word() as i32);
            assert_eq!(motion.speed_up.to_bits(), r.word());
            assert_eq!(sounds.len(), r.word() as usize);
            for sound in sounds {
                assert_eq!(sound.slot, r.word() as i32);
                assert_eq!(sound.volume.to_bits(), r.word());
                assert_eq!(sound.near_distance.to_bits(), r.word());
                assert_eq!(sound.far_distance.to_bits(), r.word());
            }
        }
    }
    assert!(r.0.is_empty());
}
