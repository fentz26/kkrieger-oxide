use kkrieger_oxide::{
    kx::{Class, Document},
    scene::{
        exec::Execution,
        overlay::{Pool, Viewport},
    },
};
#[test]
fn exported_output_count_is_supplied_by_call_abi_instead_of_parameter_block() {
    for legacy in [false, true] {
        let mut d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        d.events.clear();
        d.classes = vec![
            Class {
                id: 0x0d,
                convention: 0,
                packing: &[],
            },
            Class {
                id: 0xe6,
                convention: 0x08000104,
                packing: b"bcbc",
            },
            Class {
                id: 0xf0,
                convention: 0x08000118,
                packing: &[],
            },
            Class {
                id: 0xc1,
                convention: 0,
                packing: &[],
            },
        ];
        d.operators.truncate(4);
        for (i, op) in d.operators.iter_mut().enumerate() {
            op.class = i;
            op.animation = &[1];
            op.inputs.clear();
            op.links.clear();
            op.parameters.clear();
        }
        d.operators[0].inputs = vec![1, 1];
        d.operators[0].links = vec![Some(1)];
        d.operators[1].inputs = vec![2];
        d.operators[1].parameters = vec![5, 0xffffffff, 4, 0xffffffff];
        d.operators[2].inputs = vec![3];
        let mut p = vec![0; 24];
        p[0] = 3;
        p[1] = 3;
        p[9] = 100f32.to_bits();
        p[10] = 0.125f32.to_bits();
        p[13] = 1f32.to_bits();
        p[14] = 1f32.to_bits();
        p[16] = 90f32.to_bits();
        p[22] = 1f32.to_bits();
        p[23] = 1f32.to_bits();
        if legacy {
            p.truncate(18);
            d.classes[2].convention = 0x08000112;
        }
        d.operators[2].parameters = p;
        let mut e = Execution::new(&d);
        e.overlay = Some(Pool::new(
            Viewport {
                screen: 0,
                render_target: -1,
                window: [0, 0, 640, 360],
            },
            std::array::from_fn(|i| 100 + i as i32),
        ));
        e.frame(&d, 0).unwrap();
        assert_eq!(e.render_passes.len(), 2);
        assert_eq!(e.overlay_draws.len(), 4);
        assert_eq!(e.overlay.as_ref().unwrap().targets[10].references, 3);
        assert_eq!(d.operators[1].parameters.len(), 4);
        assert_eq!(
            d.operators[2].parameters.len(),
            if legacy { 18 } else { 24 }
        );
    }
}
