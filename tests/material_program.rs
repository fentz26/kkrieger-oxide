use kkrieger_oxide::{
    kx::Document,
    material::{self, State},
};
#[test]
fn material_register_constants_do_not_change_generated_gpu_programs() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let roots: Vec<_> = doc
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(doc.classes[op.class].id, 0xd0 | 0xd1))
        .map(|(i, _)| i)
        .collect();
    let materials = material::generate_many(&doc, &roots).unwrap();
    let mut checked = 0;
    for material in materials {
        for pass in &material.passes {
            let original = pass.shader.state.borrow().clone();
            let source = original.shader_sources().unwrap();
            let mut changed = original.clone();
            for word in &mut changed.words[24..] {
                *word ^= 0xa5a5a5a5;
            }
            assert!(original.same_gpu_program(&changed));
            let changed = changed.shader_sources().unwrap();
            assert_eq!(source.vertex, changed.vertex);
            assert_eq!(source.fragment, changed.fragment);
            assert_eq!(source.images, changed.images);
            checked += 1;
        }
    }
    assert!(checked > 100);
}
#[test]
fn render_states_and_texture_binding_identities_still_invalidate_programs() {
    let original = State::default();
    let mut changed = original.clone();
    changed.words[0] ^= 1;
    assert!(!original.same_gpu_program(&changed));
    let mut changed = original.clone();
    changed.textures[0] = Some(42);
    assert!(!original.same_gpu_program(&changed));
}
