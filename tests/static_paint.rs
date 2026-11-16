use kkrieger_oxide::{
    kx::Document,
    material::{self, Program, Usage},
    mesh::{Mesh, identity},
    prepared_mesh::PreparedMesh,
    scene::{
        culling::Plane,
        paint::{StaticInstance, View, build_static},
    },
};
use std::rc::Rc;
#[test]
fn prepared_static_mesh_expands_each_material_pass_to_its_geometry_job() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d
        .operators
        .iter()
        .position(|o| d.classes[o.class].id == 0xd0)
        .unwrap();
    let generated = material::generate_many(&d, &[root]).unwrap();
    let mut material = (*generated[0]).clone();
    let mut first = material.passes[0].clone();
    first.program = Program::Static;
    first.usage = Usage::Other;
    first.render_pass = 250;
    let mut second = first.clone();
    second.render_pass = 2;
    second.usage = Usage::Ambient;
    material.passes = vec![first, second];
    material.material11_insert = true;
    let materials = vec![Rc::new(material::Material::default()), Rc::new(material)];
    let mut mesh = Mesh::cube([1; 3], 0, [1., 1., 1., 0., 0., 0., 0., 0., 0.]);
    mesh.materials[1].pass = 10;
    let prepared =
        PreparedMesh::from_mesh(&mut mesh, &[vec![], vec![Program::Static, Program::Static]])
            .unwrap();
    assert_eq!(prepared.pass_jobs[1], [0, 0]);
    let instance = StaticInstance {
        mesh: &prepared,
        materials: &materials,
        matrix: identity(),
        pass_adjust: 2,
    };
    let view = View {
        usage_mask: u32::MAX,
        frustum: [Plane::new([0., 0., 0., 1.]); 5],
        lights: &[],
        light_capacity: 4,
    };
    let result = build_static(&instance, &view, |_, pass| 0xab00 + pass as u16).unwrap();
    assert_eq!(result.jobs.len(), 2);
    assert_eq!(
        result
            .jobs
            .iter()
            .map(|j| (j.material_slot, j.pass, j.geometry_job))
            .collect::<Vec<_>>(),
        [(1, 0, 0), (1, 1, 0)]
    );
    assert_eq!(result.jobs[0].paint.sort_key, 0x0637ab00);
    assert_eq!(result.jobs[1].paint.sort_key, 0x0e02ab01);
    assert_eq!(
        result
            .inserts
            .iter()
            .map(|i| (i.material_slot, i.render_pass))
            .collect::<Vec<_>>(),
        [(1, 6), (1, 14)]
    );
    let missing = StaticInstance {
        materials: &materials[..1],
        ..instance
    };
    assert!(build_static(&missing, &view, |_, _| 0).is_err());
}
