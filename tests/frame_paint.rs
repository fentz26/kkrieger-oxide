use kkrieger_oxide::{
    kx::Document,
    material::{self, Program, Usage},
    mesh::{Mesh, identity},
    prepared_mesh::PreparedMesh,
    scene::{
        culling::Plane,
        exec::Job,
        paint::{Asset, DrawKind, View, build_static_frame},
    },
};
use std::{collections::BTreeMap, rc::Rc};
#[test]
fn original_frame_sort_keeps_effects_before_meshes_and_reverses_job_lists() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d
        .operators
        .iter()
        .position(|o| d.classes[o.class].id == 0xd0)
        .unwrap();
    let generated = material::generate_many(&d, &[root]).unwrap();
    let mut material = (*generated[0]).clone();
    material.passes.truncate(1);
    material.passes[0].render_pass = 3;
    material.passes[0].usage = Usage::Base;
    material.passes[0].program = Program::Static;
    material.material11_insert = true;
    let material = Rc::new(material);
    d.operators[0].class = d.classes.iter().position(|c| c.id == 0x81).unwrap();
    d.operators[1].class = d.classes.iter().position(|c| c.id == 0x61).unwrap();
    d.operators[1].links = vec![Some(root)];
    let mut mesh = Mesh::cube([1; 3], 0, [1., 1., 1., 0., 0., 0., 0., 0., 0.]);
    let prepared = PreparedMesh::from_mesh(&mut mesh, &[vec![], vec![Program::Static]]).unwrap();
    let assets = BTreeMap::from([(
        0,
        Asset {
            mesh: Rc::new(prepared),
            materials: vec![Rc::new(material::Material::default()), material.clone()],
        },
    )]);
    let materials = BTreeMap::from([(root, material)]);
    let jobs = (0..4)
        .map(|i| Job::Paint {
            operator: i % 2,
            matrix: identity(),
            time: 0.,
            pass_adjust: 0,
            effect_variables: None,
        })
        .collect::<Vec<_>>();
    let view = View {
        usage_mask: u32::MAX,
        frustum: [Plane::new([0., 0., 0., 1.]); 5],
        lights: &[],
        light_capacity: 4,
    };
    let frame = build_static_frame(&d, &jobs, &assets, &materials, &view, |_, _| 0).unwrap();
    assert_eq!(
        frame.jobs.iter().map(|j| j.scene_job).collect::<Vec<_>>(),
        [3, 1, 2, 0]
    );
    assert!(frame.jobs.iter().all(|j| j.sort_key == 0x03000000));
    assert!(matches!(frame.jobs[0].kind, DrawKind::Effect));
    assert!(matches!(frame.jobs[2].kind, DrawKind::Mesh(_)));
    assert_eq!(
        frame
            .inserts
            .iter()
            .map(|i| i.scene_job)
            .collect::<Vec<_>>(),
        [2, 0]
    );
    assert!(build_static_frame(&d, &jobs, &BTreeMap::new(), &materials, &view, |_, _| 0).is_err());
}
