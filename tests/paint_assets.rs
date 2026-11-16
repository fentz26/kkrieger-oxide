use kkrieger_oxide::{kx::Document, scene::paint::prepare_assets};
use std::rc::Rc;
#[test]
fn paint_assets_share_canonical_mesh_and_default_material_allocations() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.operators.truncate(3);
    let cube = d.classes.iter().position(|c| c.id == 0x81).unwrap();
    let load = d.classes.len();
    d.classes.push(kkrieger_oxide::kx::Class {
        id: 0x01,
        convention: 0,
        packing: &[],
    });
    for op in &mut d.operators {
        op.class = cube;
        op.inputs.clear();
        op.links.clear();
        op.animation = &[1];
        op.parameters = vec![
            1,
            1,
            1,
            0,
            1f32.to_bits(),
            1f32.to_bits(),
            1f32.to_bits(),
            0,
            0,
            0,
            0,
            0,
            0,
        ];
    }
    d.operators[2].class = load;
    d.operators[2].links = vec![Some(0)];
    let assets = prepare_assets(&d, &[0, 2, 1, 0], true).unwrap();
    assert_eq!(assets.meshes.keys().copied().collect::<Vec<_>>(), [0, 1]);
    assert!(!Rc::ptr_eq(
        &assets.meshes[&0].mesh,
        &assets.meshes[&1].mesh
    ));
    assert!(Rc::ptr_eq(
        &assets.meshes[&0].materials[1],
        &assets.default_material
    ));
    assert!(Rc::ptr_eq(
        &assets.meshes[&1].materials[1],
        &assets.default_material
    ));
    assert!(
        assets
            .meshes
            .values()
            .all(|a| !a.mesh.vertices.is_empty() && a.mesh.pass_jobs[1].len() == 1)
    );
    assert_eq!(
        assets.default_material.passes[0]
            .shader
            .state
            .borrow()
            .words[28],
        0x00ff0000
    );
    assert!(prepare_assets(&d, &[99], true).is_err());
}

#[test]
#[ignore = "full authored mesh and texture CPU preparation"]
fn authored_mesh_exports_resolve_all_paint_material_slots() {
    use kkrieger_oxide::mesh_graph;
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let roots: std::collections::BTreeSet<_> = d
        .operators
        .iter()
        .filter(|o| !mesh_graph::implemented(d.classes[o.class].id))
        .flat_map(|o| o.inputs.iter().chain(o.links.iter().flatten()))
        .copied()
        .filter(|&i| mesh_graph::implemented(d.classes[d.operators[i].class].id))
        .collect();
    let roots: Vec<_> = roots.into_iter().collect();
    assert!(!roots.is_empty());
    let assets = prepare_assets(&d, &roots, true).unwrap();
    assert_eq!(assets.meshes.len(), roots.len());
    let mut vertices = 0;
    for asset in assets.meshes.values() {
        assert_eq!(asset.materials.len(), asset.mesh.materials.len());
        for (slot, material) in asset.materials.iter().enumerate() {
            assert_eq!(material.passes.len(), asset.mesh.pass_jobs[slot].len());
        }
        vertices += asset.mesh.vertices.len();
    }
    assert!(vertices > 0 && !assets.materials.is_empty());
    eprintln!(
        "authored paint meshes={} materials={} vertices={vertices}",
        assets.meshes.len(),
        assets.materials.len()
    );
}
