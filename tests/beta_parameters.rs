use kkrieger_oxide::{game::State, kx::Layout};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
}
#[test]
fn released_parameter_updates_match_native_state() {
    let mut r = Reader(include_bytes!("fixtures/beta-parameters.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 3);
    for case in 0..64 {
        let mut state = State::default();
        for call in 0..3 {
            state.reset_by_operator = r.word() != 0;
            let words: [u32; 57] = std::array::from_fn(|_| r.word());
            state
                .apply_parameters_for_layout(&words, Layout::Beta2004)
                .unwrap();
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
