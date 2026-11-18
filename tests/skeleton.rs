use kkrieger_oxide::{
    mesh::identity,
    prepared_mesh::skeleton::{Local, Node, evaluate},
};
#[test]
fn hierarchy_and_bind_pose_follow_original_multiply_order() {
    let mut root = identity();
    root[3][0] = 10.0;
    let mut child = identity();
    child[3][1] = 2.0;
    let mut bind = identity();
    bind[3][0] = -1.0;
    let nodes = [
        Node {
            parent: None,
            base_pose: identity(),
            local: Local::Static(root),
        },
        Node {
            parent: Some(0),
            base_pose: bind,
            local: Local::Static(child),
        },
    ];
    let mut temp = [identity(); 2];
    let mut output = temp;
    evaluate(&nodes, 0.0, &mut temp, &mut output, |_, _, _| {
        panic!("no spline")
    })
    .unwrap();
    assert_eq!(output[0][3], [10.0, 0.0, 0.0, 1.0]);
    assert_eq!(temp[1][3], [10.0, 2.0, 0.0, 1.0]);
    assert_eq!(output[1][3], [9.0, 2.0, 0.0, 1.0]);
}
#[test]
fn key_tracks_quantize_wrap_and_leave_missing_components_unchanged() {
    let keys = [[0.0, 0.0, 0.0], [8.0, 4.0, 2.0]];
    let nodes = [Node {
        parent: None,
        base_pose: identity(),
        local: Local::Keys {
            srt: [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 99.0, 99.0, 99.0],
            count: 2,
            tracks: [None, None, Some(&keys)],
        },
    }];
    for (time, expected) in [
        (-1.0, 0.0),
        (0.25, 4.0),
        (0.75, 4.0),
        (1.0, 0.0),
        (0.0001, 0.0),
    ] {
        let mut temp = [identity()];
        let mut output = temp;
        evaluate(
            &nodes,
            time,
            &mut temp,
            &mut output,
            |_, _, _| unreachable!(),
        )
        .unwrap();
        assert_eq!(
            output[0][3],
            [expected, expected * 0.5, expected * 0.25, 1.0]
        );
    }
}
#[test]
fn spline_receives_scaled_and_original_time_and_bad_storage_is_rejected() {
    let nodes = [Node {
        parent: None,
        base_pose: identity(),
        local: Local::Spline {
            factor: 2.0,
            offset: 0.25,
        },
    }];
    let mut temp = [identity()];
    let mut output = temp;
    evaluate(
        &nodes,
        0.5,
        &mut temp,
        &mut output,
        |index, time, original| {
            assert_eq!((index, time, original), (0, 1.25, 0.5));
            Ok(identity())
        },
    )
    .unwrap();
    assert!(
        evaluate(&nodes, f32::NAN, &mut temp, &mut output, |_, _, _| Ok(
            identity()
        ))
        .is_err()
    );
    assert!(evaluate(&nodes, 0.0, &mut [], &mut output, |_, _, _| Ok(identity())).is_err());
}

#[test]
fn forward_parent_reads_retained_temporary_matrix_and_invalid_tracks_do_not_write() {
    let mut retained = identity();
    retained[3][0] = 7.0;
    let nodes = [
        Node {
            parent: Some(1),
            base_pose: identity(),
            local: Local::Static(identity()),
        },
        Node {
            parent: None,
            base_pose: identity(),
            local: Local::Static(identity()),
        },
    ];
    let mut temp = [identity(), retained];
    let mut output = [identity(); 2];
    evaluate(
        &nodes,
        0.0,
        &mut temp,
        &mut output,
        |_, _, _| unreachable!(),
    )
    .unwrap();
    assert_eq!(output[0][3], [7.0, 0.0, 0.0, 1.0]);
    assert_eq!(output[1], identity());
    let invalid = [Node {
        parent: None,
        base_pose: identity(),
        local: Local::Keys {
            srt: [0.0; 9],
            count: 2,
            tracks: [Some(&[[0.0; 3]]), None, None],
        },
    }];
    let mut temp = [retained];
    let mut output = [retained];
    assert!(
        evaluate(
            &invalid,
            0.0,
            &mut temp,
            &mut output,
            |_, _, _| unreachable!()
        )
        .is_err()
    );
    assert_eq!(temp, [retained]);
    assert_eq!(output, [retained]);
}

#[test]
fn original_cpp_skeleton_keys_and_hierarchy_match_repeated_evaluations() {
    struct Reader<'a>(&'a [u8]);
    impl Reader<'_> {
        fn word(&mut self) -> u32 {
            let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            v
        }
        fn float(&mut self) -> f32 {
            f32::from_bits(self.word())
        }
        fn matrix(&mut self) -> [[f32; 4]; 4] {
            std::array::from_fn(|_| std::array::from_fn(|_| self.float()))
        }
    }
    let mut r = Reader(include_bytes!("fixtures/skeleton.bin"));
    assert_eq!(r.word(), 128);
    for case in 0..128 {
        let records: Vec<_> = (0..4)
            .map(|_| {
                let parent = r.word();
                let base = r.matrix();
                let mode = r.word();
                let local = r.matrix();
                let factor = r.float();
                let offset = r.float();
                let srt = std::array::from_fn(|_| r.float());
                let tracks = std::array::from_fn::<_, 3, _>(|_| {
                    let present = r.word() != 0;
                    let keys = std::array::from_fn::<_, 3, _>(|_| {
                        std::array::from_fn::<_, 3, _>(|_| r.float())
                    });
                    (present, keys)
                });
                (parent, base, mode, local, factor, offset, srt, tracks)
            })
            .collect();
        let nodes: Vec<_> = records
            .iter()
            .map(
                |(parent, base, mode, local, factor, offset, srt, tracks)| Node {
                    parent: (*parent != u32::MAX).then_some(*parent as usize),
                    base_pose: *base,
                    local: match mode {
                        0 => Local::Static(*local),
                        1 => Local::Keys {
                            srt: *srt,
                            count: 3,
                            tracks: std::array::from_fn(|i| {
                                tracks[i].0.then_some(tracks[i].1.as_slice())
                            }),
                        },
                        _ => Local::Spline {
                            factor: *factor,
                            offset: *offset,
                        },
                    },
                },
            )
            .collect();
        let mut temp = [identity(); 4];
        let mut output = temp;
        for step in 0..5 {
            let time = r.float();
            evaluate(
                &nodes,
                time,
                &mut temp,
                &mut output,
                |_, adjusted, original| {
                    let mut m = identity();
                    m[3] = [adjusted, original, adjusted - original, 1.0];
                    Ok(m)
                },
            )
            .unwrap();
            for index in 0..4 {
                for (kind, actual) in [temp[index], output[index]].iter().enumerate() {
                    let expected = r.matrix();
                    for (field, (a, b)) in actual
                        .iter()
                        .flatten()
                        .zip(expected.iter().flatten())
                        .enumerate()
                    {
                        assert_eq!(
                            a.to_bits(),
                            b.to_bits(),
                            "case {case} step {step} node {index} kind {kind} field {field}"
                        );
                    }
                }
            }
        }
    }
    assert!(r.0.is_empty());
}
