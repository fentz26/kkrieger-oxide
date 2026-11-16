use kkrieger_oxide::{
    kx::Document,
    mesh::identity,
    scene::{effects::schedule, exec::Job},
};
fn paint(operator: usize, pass_adjust: i32) -> Job {
    Job::Paint {
        operator,
        matrix: identity(),
        time: 0.,
        pass_adjust,
        effect_variables: None,
    }
}
#[test]
fn stable_effect_sort_preserves_reverse_insertion_order_and_particle_usage() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let particle = d.classes.iter().position(|c| c.id == 0x64).unwrap();
    let print = d.classes.iter().position(|c| c.id == 0x61).unwrap();
    for (i, op) in d.operators.iter_mut().take(4).enumerate() {
        op.class = particle;
        op.links = vec![Some(10 + i)];
    }
    d.operators[2].class = print;
    let jobs = vec![paint(0, 0), paint(1, 0), paint(2, 0), paint(3, 0)];
    let scheduled = schedule(&d, &jobs, |_| Some((3, 2))).unwrap();
    assert_eq!(
        scheduled.iter().map(|s| s.job).collect::<Vec<_>>(),
        [2, 3, 1, 0]
    );
    assert_eq!(
        scheduled.iter().map(|s| s.sort_key).collect::<Vec<_>>(),
        [0x03020000, 0x03070000, 0x03070000, 0x03070000]
    );
}
#[test]
fn absent_material_and_signed_pass_adjustments_follow_original_clamping() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let particle = d.classes.iter().position(|c| c.id == 0x64).unwrap();
    d.operators[0].class = particle;
    d.operators[0].links = vec![None];
    let jobs = vec![paint(0, -100), paint(0, 300), paint(0, 0)];
    let scheduled = schedule(&d, &jobs, |_| unreachable!()).unwrap();
    assert_eq!(
        scheduled.iter().map(|s| s.job).collect::<Vec<_>>(),
        [2, 0, 1]
    );
    assert_eq!(
        scheduled.iter().map(|s| s.sort_key).collect::<Vec<_>>(),
        [0x00070000, 0x00070000, 0xff070000]
    );
}
