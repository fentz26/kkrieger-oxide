use kkrieger_oxide::{gpu_scene::Resources, kx::Document};
#[test]
#[ignore = "precalculates authored gameplay paint and texture dependencies"]
fn authored_gameplay_resources_include_live_texture_slots_and_font_side_effects() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let resources = Resources::prepare(&doc, doc.roots[2].unwrap(), true, 0).unwrap();
    assert_eq!(resources.paint.meshes.len(), 169);
    assert_eq!(resources.paint.materials.len(), 67);
    for material in resources.paint.materials.values() {
        for pass in &material.passes {
            for texture in pass.shader.state.borrow().textures.into_iter().flatten() {
                assert!(
                    resources.bitmaps.contains_key(&texture),
                    "missing bitmap {texture}"
                );
            }
        }
    }
    assert!(resources.pages.contains_key(&1));
    eprintln!(
        "authored resources: {} meshes, {} materials, {} bitmaps, {} font pages",
        resources.paint.meshes.len(),
        resources.paint.materials.len(),
        resources.bitmaps.len(),
        resources.pages.len()
    );
}

#[test]
fn empty_selected_scene_has_no_unrelated_assets_and_invalid_roots_fail() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.operators.truncate(1);
    let scene = doc
        .classes
        .iter()
        .position(|class| class.id == 0xc1)
        .unwrap();
    doc.operators[0].class = scene;
    doc.operators[0].inputs.clear();
    doc.operators[0].links.clear();
    let resources = Resources::prepare(&doc, 0, true, 0).unwrap();
    assert!(resources.paint.meshes.is_empty());
    assert!(resources.paint.materials.is_empty());
    assert!(resources.bitmaps.is_empty());
    assert!(resources.pages.is_empty());
    assert!(Resources::prepare(&doc, 1, true, 0).is_err());
}

#[test]
fn viewport_simple_mode_replaces_mesh_usage_mask_without_filtering_effects() {
    use kkrieger_oxide::gpu_scene::usage_mask;
    assert_eq!(usage_mask(0, 0x1234), Some(0x1234));
    assert_eq!(usage_mask(1, 0), Some(1 << 7));
    assert_eq!(usage_mask(2, 0x1234), Some(0x1234));
    assert_eq!(usage_mask(3, u32::MAX), None);
}
