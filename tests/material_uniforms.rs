use kkrieger_oxide::material::{State, uniforms::Environment};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn floats<const N: usize>(&mut self) -> [f32; N] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
    fn matrix(&mut self) -> [[f32; 4]; 4] {
        std::array::from_fn(|_| self.floats())
    }
}
#[test]
fn original_material_set_registers_match_cpp() {
    let mut r = Reader(include_bytes!("fixtures/material-uniforms.bin"));
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let words = std::array::from_fn(|_| r.word());
        let has_srt = r.word() != 0;
        let state = State {
            words,
            textures: if has_srt {
                [Some(0), None, None, None]
            } else {
                [None; 4]
            },
        };
        let env = Environment {
            model: r.matrix(),
            view_projection: r.matrix(),
            world_to_camera: r.matrix(),
            camera: r.matrix(),
            light_position: r.floats(),
            light_color: r.floats(),
            light_amplify: f32::from_bits(r.word()),
            light_range: f32::from_bits(r.word()),
            fog_start: f32::from_bits(r.word()),
            fog_end: f32::from_bits(r.word()),
            fog_color: r.word(),
        };
        assert_eq!(state.has_texture_srt(), has_srt);
        let constants = state.constants(&env);
        assert_eq!(r.word() as usize, constants.vertex_count);
        assert_eq!(r.word() as usize, constants.pixel_count);
        for i in 0..constants.vertex_count {
            let expected = r.floats::<4>();
            // Model projection never reads c16-c19; source leaves that block uninitialized.
            if state.words[1] & 0x700 == 0x100 && (16..20).contains(&i) {
                continue;
            }
            for (a, e) in constants.uniforms.vc[i].iter().zip(expected) {
                assert!(
                    (a - e).abs() <= 2e-6,
                    "case {case}, vertex register {i}: {a} != {e}"
                );
            }
        }
        for i in 0..constants.pixel_count {
            let expected = r.floats::<4>();
            for (component, (&a, e)) in constants.uniforms.pc[i].iter().zip(expected).enumerate() {
                if i == 1 && component == 1 && state.words[2] & 7 != 0 {
                    continue;
                }
                assert!(
                    (a - e).abs() <= 2e-6,
                    "case {case}, pixel register {i}: {a} != {e}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}
