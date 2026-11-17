use kkrieger_oxide::material::{State, shader::Image};
#[test]
fn same_bitmap_in_multiple_stages_retains_independent_sampler_identity() {
    let mut state = State {
        textures: [Some(42), Some(42), None, None],
        ..Default::default()
    };
    state.words[4] = 2;
    state.words[5] = 0x100;
    let sources = state.shader_sources().unwrap();
    assert_eq!(
        sources.images,
        [
            (
                "image0".into(),
                Image::Texture {
                    operator: 42,
                    stage: 0
                }
            ),
            (
                "image1".into(),
                Image::Texture {
                    operator: 42,
                    stage: 1
                }
            ),
        ]
    );
}
