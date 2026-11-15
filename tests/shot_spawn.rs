use kkrieger_oxide::{
    game::shot::{self, Origin},
    random::Random,
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
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn optional(&mut self) -> Option<usize> {
        let v = self.word();
        (v != u32::MAX).then_some(v as usize)
    }
    fn check(&mut self, v: [f32; 4], case: usize) {
        for (a, b) in v.into_iter().zip(self.vector()) {
            assert_eq!(a.to_bits(), b.to_bits(), "case {case}: {a} vs {b}");
        }
    }
}
#[test]
fn original_shot_definitions_and_player_monster_spawns_match() {
    let mut r = Reader(include_bytes!("fixtures/shot-spawn.bin"));
    assert_eq!(r.word(), 8);
    for info in shot::DEFINITIONS {
        assert_eq!(info.mode, r.word() as i32);
        for f in [info.speed]
            .into_iter()
            .chain(info.rotation)
            .chain(info.position)
            .chain(info.size)
        {
            assert_eq!(f.to_bits(), r.word());
        }
    }
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let weapon = r.word() as usize;
        let monster = r.word() != 0;
        let seed = r.word() as i32;
        let operator = r.optional();
        let position = r.vector();
        let look = r.float();
        let direction = r.float();
        let last_camera_y = r.float();
        let camera_offset = r.float();
        let monster_direction = r.vector();
        let origin = if monster {
            Origin::Monster {
                id: 42,
                position,
                direction: monster_direction,
                cell: 17,
            }
        } else {
            Origin::Player {
                position,
                look,
                direction,
                last_camera_y,
                camera_offset,
                cell: 17,
            }
        };
        let mut rng = Random::new(seed);
        let spawn = shot::spawn(weapon, operator, origin, &mut rng).unwrap();
        assert_eq!(u32::from(spawn.is_some()), r.word());
        if let Some(s) = spawn {
            assert_eq!(s.weapon, r.word() as usize);
            assert_eq!(s.mode, r.word() as i32);
            assert_eq!(s.who, r.word());
            assert_eq!(s.double_kill, r.word() as i32);
            assert_eq!(s.cell, r.word() as usize);
            r.check(s.old_position, case);
            r.check(s.tensor, case);
            for matrix in [s.matrix, s.event_matrix] {
                for v in matrix {
                    r.check(v, case);
                }
            }
            assert_eq!(s.event_active, r.word() != 0);
            assert_eq!(s.event_operator, r.optional());
            assert_eq!(s.event_monster, r.optional());
        }
        assert_eq!(
            rng.next_u32(),
            r.word(),
            "case {case}: random stream consumption"
        );
    }
    assert!(r.0.is_empty());
}
