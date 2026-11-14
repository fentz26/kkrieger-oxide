use kkrieger_oxide::{
    animation::Vm,
    kx::{Document, Layout},
};

fn verify(data: &[u8], layout: Layout, expected: &[u8]) {
    let doc = Document::parse_layout(data, layout).unwrap();
    let mut p = 0;
    fn word(b: &[u8], p: &mut usize) -> u32 {
        let w = u32::from_le_bytes(b[*p..*p + 4].try_into().unwrap());
        *p += 4;
        w
    }
    assert_eq!(word(expected, &mut p), 5);
    assert_eq!(
        word(expected, &mut p) as usize,
        doc.operators.iter().filter(|o| o.animation != [1]).count()
    );
    for time in [0., 0.25, 1., 17.375, 1024.] {
        for (id, op) in doc
            .operators
            .iter()
            .enumerate()
            .filter(|(_, o)| o.animation != [1])
        {
            assert_eq!(word(expected, &mut p) as usize, id);
            assert_eq!(word(expected, &mut p) as usize, op.parameters.len());
            let changed = word(expected, &mut p) != 0;
            let saves = word(expected, &mut p) as usize;
            let mut vm = Vm::new();
            vm.variables[0] = [time; 4];
            vm.variables[1] = [0.75; 4];
            let mut animated = op.parameters.clone();
            let r = vm
                .execute(op, &mut animated, &doc.splines, None)
                .unwrap_or_else(|e| panic!("op {id}: {e}"));
            assert_eq!(r.changed, changed, "changed flag at op {id} / time {time}");
            assert_eq!(r.saved_variables, saves);
            let mut float_words = vec![false; animated.len()];
            for (i, &format) in doc.classes[op.class].packing.iter().enumerate() {
                float_words[i] = matches!(format.to_ascii_lowercase(), b'g' | b'f' | b'e');
            }
            for (i, &actual) in animated.iter().enumerate() {
                let expected = word(expected, &mut p);
                if actual == expected {
                    continue;
                }
                assert!(
                    float_words[i],
                    "integer mismatch op {id} word {i}: {actual:x} != {expected:x}"
                );
                let a = f32::from_bits(actual);
                let b = f32::from_bits(expected);
                assert!(
                    a.is_finite() && b.is_finite() && (a - b).abs() <= 2e-6 * b.abs().max(1.),
                    "op {id}, time {time}, word {i}: Rust {a} vs original C++ {b}"
                );
            }
            vm.restore(r.saved_variables).unwrap();
        }
    }
    assert_eq!(p, expected.len());
}

#[test]
fn published_animation_matches_original_cpp_vm() {
    verify(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
        include_bytes!("fixtures/animation3383.bin"),
    );
}

#[test]
#[ignore = "requires independently extracted beta and its local C++ oracle output"]
fn original_beta_animation_matches_original_cpp_vm() {
    verify(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
        &std::fs::read(".reference/animation-beta.bin").unwrap(),
    );
}
