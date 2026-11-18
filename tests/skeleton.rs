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
