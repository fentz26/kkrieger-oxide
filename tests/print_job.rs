use kkrieger_oxide::{
    bitmap::{FontPage, Letter},
    kx::Document,
    scene::{camera::Environment, exec::Execution, print::Context},
};
use std::collections::BTreeMap;
fn document() -> Document<'static> {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.operators.truncate(2);
    let scene = d.classes.iter().position(|c| c.id == 0xc1).unwrap();
    let print = d.classes.iter().position(|c| c.id == 0x61).unwrap();
    for op in &mut d.operators {
        op.inputs.clear();
        op.links.clear();
        op.animation = &[1];
    }
    d.operators[0].class = scene;
    d.operators[0].inputs = vec![1];
    d.operators[1].class = print;
    d.operators[1].animation = &[2, 5, 0x91, 1, 1];
    d.operators[1].parameters = vec![0x34, 1f32.to_bits(), 1f32.to_bits(), 0, 0, 0];
    d.operators[1].strings = vec![b"%2l;"];
    d
}
#[test]
fn print_job_uses_captured_animation_inventory_and_selected_atlas() {
    let d = document();
    let mut e = Execution::new(&d);
    e.vm.variables[5] = [2.; 4];
    e.game.inventory.amounts[0] = 7;
    e.frame(&d, 0).unwrap();
    let job = e.jobs[0].clone();
    e.vm.variables[5] = [9.; 4];
    let page = FontPage {
        page: 3,
        line_height: 16,
        letters: [Letter {
            uv: [0., 0., 1., 1.],
            width: 1.,
            ..Letter::default()
        }; 256],
    };
    let pages = BTreeMap::from([(3, page)]);
    let camera = Environment::default();
    let context = Context {
        camera: &camera,
        pages: &pages,
        material_passes: Some(1),
        fps: 60.,
    };
    let draw = e.draw_print_job(&d, &job, &context).unwrap().unwrap();
    assert_eq!(draw.page, 3);
    assert_eq!(draw.geometry.vertices.len(), 8);
    assert_eq!(draw.geometry.vertices[1][0], 2.);
    assert_eq!(draw.environment.orthogonal, 1);
    assert_eq!(draw.environment.model[3], [-1., 1., 0., 1.]);
    let empty = BTreeMap::new();
    let context = Context {
        pages: &empty,
        material_passes: None,
        ..context
    };
    assert!(e.draw_print_job(&d, &job, &context).unwrap().is_none());
}
#[test]
fn missing_atlas_error_restores_local_animation_variables() {
    let mut d = document();
    d.operators[1].animation = &[24, 0, 0, 128, 63, 0x81, 7, 1];
    let mut e = Execution::new(&d);
    e.vm.variables[7] = [8.; 4];
    e.frame(&d, 0).unwrap();
    let job = e.jobs[0].clone();
    let camera = Environment::default();
    let pages = BTreeMap::new();
    let context = Context {
        camera: &camera,
        pages: &pages,
        material_passes: Some(1),
        fps: 60.,
    };
    assert!(e.draw_print_job(&d, &job, &context).is_err());
    assert_eq!(e.vm.variables[7], [8.; 4]);
}
