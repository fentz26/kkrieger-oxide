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

#[test]
fn native_original_mesh_pass_expansion_matches_jobs_and_insert_eligibility() {
    let mut bytes: &[u8] = include_bytes!("fixtures/mesh-paint.bin");
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
        let usage = word(&mut bytes);
        let animated = word(&mut bytes) != 0;
        let visible = word(&mut bytes) != 0;
        let mask = word(&mut bytes);
        let material_key = word(&mut bytes) as u16;
        let capacity = word(&mut bytes) as usize;
        let render_pass = word(&mut bytes) as i32;
        let mesh_pass = word(&mut bytes) as i32;
        let adjust = word(&mut bytes) as i32;
        let mut lights = Vec::new();
        for _ in 0..4 {
            let position = std::array::from_fn(|_| real(&mut bytes));
            let range = real(&mut bytes);
            let flags = word(&mut bytes);
            let shadow = real(&mut bytes);
            let count = word(&mut bytes) as usize;
            let zfail = real(&mut bytes);
            lights.push(Light {
                position,
                range,
                flags,
                shadow_planes: [Plane::new([0., 0., 0., shadow]); 5],
                zfail_planes: vec![Plane::new([0., 0., 0., zfail]); count],
            });
        }
        let candidate = Candidate {
            animated,
            material_key,
            render_pass,
            mesh_pass,
            pass_adjust: adjust,
            ..candidate(usage)
        };
        let view = View {
            usage_mask: mask,
            frustum: [Plane::new([0., 0., 0., if visible { 1. } else { -1. }]); 5],
            lights: &lights,
            light_capacity: capacity,
        };
        let result = build(&candidate, &view).unwrap();
        assert_eq!(
            result.render_pass.map_or(-1, i32::from),
            word(&mut bytes) as i32,
            "case {case} insert pass"
        );
        assert_eq!(
            result.jobs.len(),
            word(&mut bytes) as usize,
            "case {case} job count"
        );
        for job in result.jobs {
            assert_eq!(job.sort_key, word(&mut bytes), "case {case} sort key");
            assert_eq!(
                job.light.map_or(-1, |i| i as i32),
                word(&mut bytes) as i32,
                "case {case} light"
            );
            assert_eq!(job.zfail, word(&mut bytes) != 0, "case {case} stencil mode");
        }
    }
    assert!(bytes.is_empty());
}
