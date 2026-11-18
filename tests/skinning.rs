use kkrieger_oxide::{
    mesh::identity,
    prepared_mesh::{
        Vertex,
        skinning::{Influence, deform},
    },
};
fn vertex() -> Vertex {
    Vertex {
        position: [1.0, 2.0, 3.0],
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 1.0],
        color: 0x12345678,
        uv: [0.25, 0.75],
    }
}
#[test]
fn weighted_affine_position_and_orthonormal_frame_preserve_uv_and_color() {
    let mut a = identity();
    a[3] = [2.0, 4.0, 6.0, 1.0];
    let mut b = identity();
    b[3] = [-2.0, -4.0, -6.0, 1.0];
    let source = [vertex()];
    let influences = [Influence {
        count: 2,
        weights: [0.75, 0.25, 0.0, 0.0],
        matrices: [0, 1, 0, 0],
    }];
    let mut output = [vertex()];
    deform(&source, &influences, &[a, b], &mut output).unwrap();
    assert_eq!(output[0].position, [2.0, 4.0, 6.0]);
    assert_eq!(output[0].normal, [0.0, 0.0, 1.0]);
    assert_eq!(output[0].tangent, [1.0, 0.0, 0.0]);
    assert_eq!(output[0].uv, source[0].uv);
    assert_eq!(output[0].color, source[0].color);
}
#[test]
fn zero_influences_preserve_zero_frame_and_invalid_bindings_do_not_write() {
    let source = [vertex()];
    let mut output = [vertex()];
    deform(&source, &[Influence::default()], &[], &mut output).unwrap();
    assert_eq!(output[0].position, [0.0; 3]);
    assert_eq!(output[0].normal, [0.0; 3]);
    assert_eq!(output[0].tangent, [0.0; 3]);
    let invalid = Influence {
        count: 1,
        weights: [1.0; 4],
        matrices: [9; 4],
    };
    output[0] = vertex();
    assert!(deform(&source, &[invalid], &[identity()], &mut output).is_err());
    assert_eq!(output[0].position, source[0].position);
    assert!(deform(&source, &[], &[], &mut output).is_err());
    assert!(
        deform(
            &source,
            &[Influence {
                count: 5,
                ..invalid
            }],
            &[],
            &mut output
        )
        .is_err()
    );
}

#[test]
fn nonuniform_bone_scale_renormalizes_frame_without_renormalizing_weights() {
    let mut matrix = identity();
    matrix[0][0] = 2.0;
    matrix[1][1] = 3.0;
    matrix[2][2] = 4.0;
    let influence = Influence {
        count: 1,
        weights: [0.5, 0.0, 0.0, 0.0],
        matrices: [0; 4],
    };
    let source = [vertex()];
    let mut output = [vertex()];
    deform(&source, &[influence], &[matrix], &mut output).unwrap();
    assert_eq!(output[0].position, [1.0, 3.0, 6.0]);
    assert_eq!(output[0].normal, [0.0, 0.0, 1.0]);
    assert_eq!(output[0].tangent, [1.0, 0.0, 0.0]);
    let mut matrix = identity();
    matrix[0] = [0.0, 1.0, 0.0, 0.0];
    matrix[1] = [-1.0, 0.0, 0.0, 0.0];
    deform(
        &source,
        &[Influence {
            weights: [1.0; 4],
            ..influence
        }],
        &[matrix],
        &mut output,
    )
    .unwrap();
    assert_eq!(output[0].position, [-2.0, 1.0, 3.0]);
    assert_eq!(output[0].tangent, [0.0, 1.0, 0.0]);
}
