use kkrieger_oxide::{
    mesh::identity,
    scene::{
        culling::{Bounds, Plane},
        paint::{Candidate, Light, View, build},
    },
};
fn candidate(usage: u32) -> Candidate {
    Candidate {
        bounds: Bounds([[-1.; 3], [1.; 3]]),
        matrix: identity(),
        animated: false,
        usage,
        render_pass: 250,
        mesh_pass: 10,
        pass_adjust: 2,
        material_key: 0xabcd,
    }
}
#[test]
fn original_mesh_pass_wraps_before_adjust_and_duplicates_per_light() {
    let planes = [Plane::new([0., 0., 0., 1.]); 5];
    let lights = [
        Light {
            position: [0.; 3],
            range: 2.,
            flags: 2,
            shadow_planes: planes,
            zfail_planes: vec![],
        },
        Light {
            position: [100.; 3],
            range: 1.,
            flags: 2,
            shadow_planes: planes,
            zfail_planes: vec![],
        },
    ];
    let view = View {
        usage_mask: u32::MAX,
        frustum: planes,
        lights: &lights,
        light_capacity: 4,
    };
    let result = build(&candidate(4), &view).unwrap();
    assert_eq!(result.render_pass, Some(6));
    assert_eq!(result.jobs.len(), 1);
    assert_eq!(result.jobs[0].sort_key, 0x0604abcd);
    assert_eq!(result.jobs[0].light, Some(0));
    assert!(result.jobs[0].zfail);
    let result = build(&candidate(7), &view).unwrap();
    assert_eq!(result.jobs[0].sort_key, 0x0637abcd);
    assert_eq!(result.jobs[0].light, None);
    assert!(!result.jobs[0].zfail);
}
#[test]
fn original_shadow_jobs_survive_camera_culling_but_require_light_shadow_flag() {
    let visible = [Plane::new([0., 0., 0., 1.]); 5];
    let outside = [Plane::new([0., 0., 0., -1.]); 5];
    let mut lights = [Light {
        position: [0.; 3],
        range: 2.,
        flags: 0,
        shadow_planes: visible,
        zfail_planes: outside.to_vec(),
    }];
    let view = View {
        usage_mask: u32::MAX,
        frustum: outside,
        lights: &lights,
        light_capacity: 4,
    };
    assert_eq!(build(&candidate(4), &view).unwrap().render_pass, None);
    assert!(build(&candidate(3), &view).unwrap().jobs.is_empty());
    lights[0].flags = 2;
    let view = View {
        usage_mask: u32::MAX,
        frustum: outside,
        lights: &lights,
        light_capacity: 4,
    };
    let result = build(&candidate(3), &view).unwrap();
    assert_eq!(result.jobs.len(), 1);
    assert!(!result.jobs[0].zfail);
    let mut animated = candidate(4);
    animated.animated = true;
    assert_eq!(build(&animated, &view).unwrap().jobs.len(), 1);
    let masked = View {
        usage_mask: 0,
        ..view
    };
    assert_eq!(build(&animated, &masked).unwrap().render_pass, None);
}
