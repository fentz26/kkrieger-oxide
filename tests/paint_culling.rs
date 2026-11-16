use kkrieger_oxide::{
    mesh::identity,
    scene::culling::{Bounds, Frustum, Plane},
};
#[test]
fn original_bounds_transform_and_sphere_boundary() {
    let bounds = Bounds([[1., 2., 3.], [4., 6., 8.]]);
    let matrix = [
        [-2., 0., 0., 0.],
        [0., 3., 0., 0.],
        [0., 0., -1., 0.],
        [10., 20., 30., 1.],
    ];
    let world = bounds.transformed(matrix);
    assert_eq!(world.0, [[2., 26., 22.], [8., 38., 27.]]);
    assert!(world.intersects_sphere([10., 38., 27.], 2.));
    assert!(!world.intersects_sphere([10.01, 38., 27.], 2.));
    assert!(world.intersects_sphere([10., 38., 27.], -2.));
}
#[test]
fn original_culling_retains_epsilon_boundary_and_optional_far_plane() {
    let plane = Plane::new([1., 0., 0., 0.]);
    assert!(!Bounds([[-1e-6, 0., 0.]; 2]).culled(&[plane]));
    assert!(Bounds([[-2e-6, 0., 0.]; 2]).culled(&[plane]));
    let frustum = Frustum::from_view_projection(identity(), [-1., -1., 1., 1.]);
    assert_eq!(frustum.count, 6);
    assert!(!Bounds([[-0.5, -0.5, 0.25], [0.5, 0.5, 0.75]]).culled(&frustum.planes));
    assert!(Bounds([[2., 0., 0.5], [3., 1., 0.75]]).culled(&frustum.planes));
    let projection = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 1.],
        [0., 0., -0.125, 0.],
    ];
    assert_eq!(
        Frustum::from_view_projection(projection, [-1., -1., 1., 1.]).count,
        5
    );
}
