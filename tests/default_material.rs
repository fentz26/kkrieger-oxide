use kkrieger_oxide::material::{self, Program, Usage};
#[test]
fn original_overlay_default_material_matches_intro_and_tool_definitions() {
    for intro in [true, false] {
        let material = material::original_default(intro);
        assert_eq!(material.passes.len(), 1);
        let pass = &material.passes[0];
        assert_eq!(pass.usage, Usage::Light);
        assert_eq!(pass.program, Program::Static);
        assert_eq!(pass.render_pass, 0);
        assert_eq!((pass.size, pass.aspect), (1., 1.));
        let state = pass.shader.state.borrow();
        assert_eq!(state.words[0], 0x300);
        assert!(state.textures.iter().all(Option::is_none));
        if intro {
            assert_eq!(state.words[28], 0x00ff0000);
            assert_eq!(state.words[9], 0x10);
            assert_eq!(state.words[2], 0);
        } else {
            assert_eq!(state.words[28], 0x00c0c0c0);
            assert_eq!(state.words[29], 0x00404040);
            assert_eq!(state.words[32], 32f32.to_bits());
            assert_eq!(state.words[2], 5);
            assert_eq!(&state.words[8..10], &[0x10, 0x40]);
        }
        state.shader_sources().unwrap();
    }
}
