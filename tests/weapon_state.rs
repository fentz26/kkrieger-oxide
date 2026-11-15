use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{
        Events, Inventory, Parameters,
        player::Motion,
        weapon::{self, State},
    },
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
    fn optional(&mut self) -> Option<usize> {
        let v = self.word();
        (v != u32::MAX).then_some(v as usize)
    }
}
#[test]
fn original_weapon_frames_preserve_ammo_timing_switches_and_requests() {
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
    let mut r = Reader(include_bytes!("fixtures/weapon-state.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 16);
    let mut shots = 0;
    let mut dry = 0;
    for case in 0..64 {
        let mut parameters = Parameters::default();
        let mut events = Events::default();
        for i in 0..8 {
            parameters.weapon_cool[i] = r.float();
            parameters.weapon_flags[i] = r.word() as i32;
            events.shot[i] = r.optional();
            events.optics[i] = r.optional();
        }
        let mut state = State {
            current: r.word() as usize,
            next: r.word() as usize,
            cooldown: r.float(),
            fire_key: r.word() != 0,
            timer: r.float(),
            ..Default::default()
        };
        let mut motion = Motion::spawn(&world, [0., 0., 0., 1.], 0.).unwrap();
        motion.step_foot = r.word() as i32;
        motion.step_timer = r.float();
        let mut inventory = Inventory::default();
        for i in 0..4 {
            inventory.amounts[4 + i] = r.word() as i32;
        }
        let mut resets = 0;
        let mut optics = None;
        for frame in 0..16 {
            let slices = r.word() as i32;
            motion.accel_forward = r.float();
            motion.accel_side = r.float();
            motion.on_ground = r.word() != 0;
            match r.word() {
                1 => state.fire_key = true,
                2 => state.fire_key = false,
                _ => {}
            }
            if state.finish_switch() {
                resets += 1;
                optics = events.optics[state.current];
            }
            state.animate(slices, motion.accel_forward);
            let step = weapon::step(&mut motion, slices);
            let (shot, empty) = state
                .fire(&mut inventory, &parameters, &events, slices)
                .unwrap();
            let sounds: Vec<_> = step.into_iter().chain(empty).collect();
            assert_eq!(state.current, r.word() as usize, "{case}/{frame}");
            assert_eq!(state.next, r.word() as usize);
            assert_eq!(state.cooldown.to_bits(), r.word(), "{case}/{frame}");
            assert_eq!(state.fire_key, r.word() != 0);
            assert_eq!(state.timer.to_bits(), r.word(), "{case}/{frame}");
            assert_eq!(motion.step_foot, r.word() as i32);
            assert_eq!(motion.step_timer.to_bits(), r.word());
            for i in 0..4 {
                assert_eq!(inventory.amounts[4 + i], r.word() as i32);
            }
            assert_eq!(resets, r.word());
            assert_eq!(optics, r.optional());
            assert_eq!(sounds.len(), r.word() as usize);
            for sound in sounds {
                assert_eq!(sound.slot, r.word() as i32);
                assert_eq!(sound.volume.to_bits(), r.word());
                assert_eq!(sound.near_distance, 0.2);
                assert_eq!(sound.far_distance, 1.);
                if sound.slot == 15 {
                    dry += 1;
                }
            }
            assert_eq!(usize::from(shot.is_some()), r.word() as usize);
            if let Some(shot) = shot {
                assert_eq!(shot.weapon, r.word() as usize);
                assert_eq!(shot.operator, r.word() as usize);
                shots += 1;
            }
        }
    }
    assert!(shots > 0 && dry > 0);
    assert!(r.0.is_empty());
}
