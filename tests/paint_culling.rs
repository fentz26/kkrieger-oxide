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

#[test]
fn native_original_transformed_bounds_planes_and_culling_match() {
    let mut bytes: &[u8] = include_bytes!("fixtures/paint-culling.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    fn real(bytes: &mut &[u8]) -> f32 {
        f32::from_bits(word(bytes))
    }
    assert_eq!(word(&mut bytes), 512);
    for case in 0..512 {
        let bounds = Bounds(std::array::from_fn(|_| {
            std::array::from_fn(|_| real(&mut bytes))
        }));
        let matrix = std::array::from_fn(|_| std::array::from_fn(|_| real(&mut bytes)));
        let projection = std::array::from_fn(|_| std::array::from_fn(|_| real(&mut bytes)));
        let viewport = std::array::from_fn(|_| real(&mut bytes));
        let center = std::array::from_fn(|_| real(&mut bytes));
        let radius = real(&mut bytes);
        let world = bounds.transformed(matrix);
        let frustum = Frustum::from_view_projection(projection, viewport);
        for v in world.0.into_iter().flatten() {
            assert_eq!(v.to_bits(), word(&mut bytes), "case {case} bounds");
        }
        for plane in frustum.planes {
            for v in plane.equation {
                assert_eq!(v.to_bits(), word(&mut bytes), "case {case} plane");
            }
            assert_eq!(u32::from(plane.near_vertex), word(&mut bytes));
        }
        assert_eq!(frustum.count, word(&mut bytes) as usize);
        assert_eq!(
            world.intersects_sphere(center, radius),
            word(&mut bytes) != 0,
            "case {case} sphere"
        );
        assert_eq!(
            world.culled(&frustum.planes[..5]),
            word(&mut bytes) != 0,
            "case {case} five-plane cull"
        );
        assert_eq!(
            world.culled(&frustum.planes[..frustum.count]),
            word(&mut bytes) != 0,
            "case {case} frustum cull"
        );
    }
    assert!(bytes.is_empty());
}

#[test]
fn original_zfail_volume_closes_between_light_and_camera_near_plane() {
    let viewport = [-1., -1., 1., 1.];
    let behind = Frustum::zfail_volume([0., 0., 0.], identity(), 1., [1., 1.], viewport);
    assert_eq!(behind.count, 6);
    assert!(!Bounds([[-0.1, -0.1, 0.4], [0.1, 0.1, 0.6]]).culled(&behind.planes[..behind.count]));
    assert!(Bounds([[-0.1, -0.1, 1.4], [0.1, 0.1, 1.6]]).culled(&behind.planes[..behind.count]));
    let front = Frustum::zfail_volume([0., 0., 2.], identity(), 1., [1., 1.], viewport);
    assert!(!Bounds([[-0.1, -0.1, 1.4], [0.1, 0.1, 1.6]]).culled(&front.planes[..front.count]));
    let on = Frustum::zfail_volume([0., 0., 1.], identity(), 1., [1., 1.], viewport);
    assert_eq!(on.count, 2);
    assert_eq!(on.planes[0].equation, [0., 0., 4., -4. + 1e-4]);
    assert_eq!(on.planes[1].equation, [-0., -0., -4., 4. - 1e-4]);
}
#[test]
fn original_frustum_enlargement_and_normalization_preserve_selectors() {
    let mut frustum = Frustum::from_view_projection(identity(), [-1., -1., 1., 1.]);
    frustum.enlarge_to_include([2., 0., 0.5]);
    assert!(!Bounds([[2., 0., 0.5]; 2]).culled(&frustum.planes));
    let selectors = frustum.planes.map(|p| p.near_vertex);
    frustum.normalize();
    assert_eq!(frustum.planes.map(|p| p.near_vertex), selectors);
}
