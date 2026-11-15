use kkrieger_oxide::game::{Events, State};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn optional(&mut self) -> Option<usize> {
        let v = self.word();
        (v != u32::MAX).then_some(v as usize)
    }
}
#[test]
fn original_game_parameters_preserve_reset_and_live_tuning_semantics() {
    let mut r = Reader(include_bytes!("fixtures/game-parameters.bin"));
    assert_eq!(r.word(), 128);
    assert_eq!(r.word(), 3);
    for case in 0..128 {
        let mut state = State::default();
        for call in 0..3 {
            state.reset_by_operator = r.word() != 0;
            let words: [u32; 57] = std::array::from_fn(|_| r.word());
            state.apply_parameters(&words).unwrap();
            assert_eq!(state.reset_by_operator, r.word() != 0);
            for actual in state
                .inventory
                .amounts
                .into_iter()
                .chain(state.inventory.maxima)
                .chain(state.inventory.weapons)
            {
                assert_eq!(actual as u32, r.word(), "{case}/{call}");
            }
            let p = &state.parameters;
            for f in [
                p.gravity,
                p.damp_ground,
                p.damp_air,
                p.damp_height,
                p.stair_height,
                p.eye_height,
                p.mouse_turn,
                p.mouse_look,
                p.accel_side,
                p.accel_forward,
                p.accel_back,
                p.jump_force,
            ]
            .into_iter()
            .chain(p.start_position)
            .chain([p.damp_camera])
            {
                assert_eq!(f.to_bits(), r.word(), "{case}/{call}");
            }
            for v in p.weapon_hits {
                assert_eq!(v as u32, r.word());
            }
            for v in p.weapon_cool {
                assert_eq!(v.to_bits(), r.word());
            }
            for v in p.weapon_flags {
                assert_eq!(v as u32, r.word());
            }
            assert_eq!(p.camera_offset.to_bits(), r.word());
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn original_weapon_event_registrations_preserve_group_and_pointer_identity() {
    let mut r = Reader(include_bytes!("fixtures/game-events.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 6);
    for _ in 0..64 {
        let mut events = Events::default();
        for _ in 0..6 {
            let kind = r.word();
            let links: [Option<usize>; 8] = std::array::from_fn(|_| r.optional());
            events.apply(kind, &links).unwrap();
            for actual in events
                .shot
                .into_iter()
                .chain(events.optics)
                .chain(events.explode.into_iter().flatten())
            {
                assert_eq!(actual, r.optional());
            }
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn scene_handlers_apply_parameters_and_register_events_without_executing_links() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::{Execution, Job},
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.classes = vec![
        Class {
            id: 0x10,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0x12,
            convention: 0x1000_0000,
            packing: &[],
        },
        Class {
            id: 0xc4,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(3);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![0; 57];
    doc.operators[0].parameters[0] = 10;
    doc.operators[0].parameters[1] = 100;
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![2];
    doc.operators[1].inputs = vec![2];
    doc.operators[1].links = vec![Some(2); 8];
    doc.operators[2].class = 2;
    doc.operators[2].parameters = vec![0; 11];
    let mut execution = Execution::new(&doc);
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.game.inventory.amounts[0], 10);
    assert_eq!(execution.game.inventory.maxima[0], 100);
    assert_eq!(execution.game.events.explode[0], [Some(2); 8]);
    assert_eq!(execution.game.events.shot, [None; 8]);
    assert!(matches!(
        execution.jobs.as_slice(),
        [Job::Light { operator: 2, .. }]
    ));
    execution.game.inventory.amounts[0] = 3;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.game.inventory.amounts[0], 3);
    execution.game.reset_by_operator = true;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.game.inventory.amounts[0], 10);
}
