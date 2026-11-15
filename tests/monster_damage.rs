use kkrieger_oxide::game::{
    State,
    monster::Health,
    shot::{Damage, Explosion, Outcome},
};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
}
#[test]
fn original_monster_damage_preserves_dead_guard_activation_and_switch_wrap() {
    let mut r = Reader(include_bytes!("fixtures/monster-damage.bin"));
    assert_eq!(r.word(), 256);
    assert_eq!(r.word(), 4);
    for case in 0..256 {
        let mut health = Health {
            life: r.word() as i32,
            armor: r.word() as i32,
            state: r.word() as i32,
            kill_switch: r.word() as i32,
        };
        let watch = r.word() as usize;
        let mut switches = [0; 256];
        switches[watch] = r.word() as u8;
        for call in 0..4 {
            let hits = r.word() as i32;
            health.hit(hits, &mut switches).unwrap();
            assert_eq!(health.life, r.word() as i32, "{case}/{call}");
            assert_eq!(health.armor, r.word() as i32);
            assert_eq!(health.state, r.word() as i32);
            assert_eq!(switches[watch], r.word() as u8);
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn projectile_damage_is_applied_before_explosion_is_released() {
    let mut game = State::default();
    let actor = game.monsters.allocate().unwrap();
    actor.borrow_mut().health = Health {
        life: 1,
        armor: 0,
        state: 3,
        kill_switch: 7,
    };
    game.monsters.register(actor.borrow().id).unwrap();
    let mut switches = [0; 256];
    switches[7] = 255;
    let result = game
        .apply_shot_outcome(
            Outcome {
                alive: false,
                damage: Some(Damage::Monster {
                    monster: 0,
                    hits: 10,
                }),
                explosion: Some(Explosion {
                    operator: Some(55),
                    matrix: kkrieger_oxide::mesh::identity(),
                }),
            },
            &mut switches,
        )
        .unwrap();
    assert_eq!(actor.borrow().health.life, 0);
    assert_eq!(actor.borrow().health.state, 4);
    assert_eq!(switches[7], 0);
    assert_eq!(result.explosion.unwrap().operator, Some(55));
    assert!(result.sound.is_none());
    assert!(!result.alive);
    game.inventory.amounts[0] = 20;
    let result = game
        .apply_shot_outcome(
            Outcome {
                alive: false,
                damage: Some(Damage::Player { hits: 10 }),
                explosion: None,
            },
            &mut switches,
        )
        .unwrap();
    assert_eq!(game.inventory.amounts[0], 10);
    let sound = result.sound.unwrap();
    assert_eq!(sound.slot, 10);
    assert_eq!(sound.far_distance, 160.);
    assert!(result.explosion.is_none());
}
