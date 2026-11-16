use kkrieger_oxide::{
    kx::Document,
    scene::exec::{Execution, Job},
};
#[test]
fn effect_jobs_capture_contiguous_variable_range_including_dependencies() {
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.operators.truncate(3);
    let scene_class = d.classes.iter().position(|c| c.id == 0xc1).unwrap();
    let effect_class = d.classes.iter().position(|c| c.id == 0x64).unwrap();
    for o in &mut d.operators {
        o.inputs.clear();
        o.links.clear();
        o.animation = &[1];
        o.parameters = vec![0; 12];
    }
    d.operators[0].class = scene_class;
    d.operators[0].inputs = vec![1];
    d.operators[1].class = effect_class;
    d.operators[1].inputs = vec![2];
    d.operators[1].animation = &[2, 5, 0x91, 2, 1];
    d.operators[2].animation = &[2, 9, 0x91, 2, 1];
    let mut execution = Execution::new(&d);
    for (i, variable) in execution.vm.variables.iter_mut().enumerate() {
        *variable = [i as f32; 4];
    }
    execution.frame(&d, 0).unwrap();
    let Job::Paint {
        effect_variables: Some(snapshot),
        ..
    } = &execution.jobs[0]
    else {
        panic!("missing effect snapshot")
    };
    assert_eq!(snapshot.start, 5);
    assert_eq!(
        snapshot.values,
        (5..10).map(|i| [i as f32; 4]).collect::<Vec<_>>()
    );
    execution.vm.variables[7] = [100.; 4];
    assert_eq!(snapshot.values[2], [7.; 4]);
    assert_eq!(execution.jobs.len(), 1);
}

#[test]
fn variable_range_skips_uncalled_links_and_implicit_matrix_reads() {
    use kkrieger_oxide::animation::effect_variable_range;
    let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    d.operators.truncate(2);
    for o in &mut d.operators {
        o.inputs.clear();
        o.links.clear();
        o.animation = &[1];
    }
    d.operators[0].links = vec![Some(1)];
    d.operators[1].animation = &[2, 31, 0x91, 0, 1];
    let class = d.operators[0].class;
    d.classes[class].convention |= 0x1000_0000;
    assert_eq!(effect_variable_range(&d, 0).unwrap(), None);
    d.classes[class].convention &= !0x1000_0000;
    assert_eq!(effect_variable_range(&d, 0).unwrap(), Some(31..32));
    d.operators[0].links.clear();
    d.operators[0].animation = &[24, 0, 0, 0, 0, 27, 0x91, 0, 1];
    assert_eq!(effect_variable_range(&d, 0).unwrap(), None);
}
