use kkrieger_oxide::scene::paint::{
    DrawKind, FrameInsert, FramePlan, InsertUse, MeshPaint, Paint, Queued,
    render::{Action::*, actions},
};
fn mesh(key: u32, light: Option<usize>) -> Queued {
    Queued {
        scene_job: 0,
        sort_key: key,
        kind: DrawKind::Mesh(MeshPaint {
            material_slot: 1,
            pass: 0,
            geometry_job: 0,
            paint: Paint {
                sort_key: key,
                light,
                zfail: false,
            },
        }),
    }
}
#[test]
fn original_insert_and_light_transitions_finish_before_the_next_group() {
    let frame = FramePlan {
        jobs: vec![
            mesh(0, None),
            mesh(0x00030000, Some(0)),
            mesh(0x00040000, Some(0)),
            mesh(0x00130000, Some(1)),
            mesh(0x00140000, Some(1)),
            mesh(0x00f50000, None),
            Queued {
                scene_job: 1,
                sort_key: 0x01070000,
                kind: DrawKind::Effect,
            },
        ],
        inserts: vec![FrameInsert {
            scene_job: 0,
            insert: InsertUse {
                material_slot: 1,
                render_pass: 0,
            },
        }],
    };
    let mut grab = [false; 256];
    grab[1] = true;
    assert_eq!(
        actions(&frame, &grab, true),
        [
            ClearScissor,
            Draw(0),
            ClearScissor,
            ClearDestinationAlpha,
            ApplyViewProjection,
            SetLight(0),
            Draw(1),
            ClearScissor,
            ResetShadowStates,
            SetLight(0),
            Draw(2),
            ClearScissor,
            ClearStencil,
            ClearScissor,
            SetLight(1),
            Draw(3),
            ClearScissor,
            ResetShadowStates,
            SetLight(1),
            Draw(4),
            ClearScissor,
            ClearStencil,
            ClearScissor,
            ClearScissor,
            Draw(5),
            ClearScissor,
            AddDestinationAlpha,
            ApplyViewProjection,
            GrabCurrentRender,
            Draw(6),
            ApplyViewProjection,
            ClearScissor,
        ]
    );
}
#[test]
fn same_group_keeps_state_and_empty_frames_still_release_scissor() {
    let frame = FramePlan {
        jobs: vec![mesh(0x00140000, Some(1)), mesh(0x00140001, Some(1))],
        inserts: vec![],
    };
    assert_eq!(
        actions(&frame, &[false; 256], true),
        [ClearScissor, SetLight(1), Draw(0), Draw(1), ClearScissor]
    );
    assert_eq!(
        actions(
            &FramePlan {
                jobs: vec![],
                inserts: vec![]
            },
            &[false; 256],
            true
        ),
        [ClearScissor]
    );
}
#[test]
fn final_material_usage_hook_runs_without_a_following_job() {
    let frame = FramePlan {
        jobs: vec![mesh(0, None)],
        inserts: vec![FrameInsert {
            scene_job: 0,
            insert: InsertUse {
                material_slot: 1,
                render_pass: 0,
            },
        }],
    };
    assert_eq!(
        actions(&frame, &[false; 256], true),
        [
            ClearScissor,
            Draw(0),
            ClearScissor,
            ClearDestinationAlpha,
            ApplyViewProjection
        ]
    );
    assert_eq!(
        actions(&frame, &[false; 256], false),
        [ClearScissor, Draw(0), ClearScissor, ApplyViewProjection]
    );
}

#[test]
fn render_actions_match_unchanged_cpp_loop_and_material11_hooks() {
    let mut data = include_bytes!("fixtures/paint-render.bin").as_slice();
    fn word(data: &mut &[u8]) -> u32 {
        let (value, rest) = data.split_at(4);
        *data = rest;
        u32::from_le_bytes(value.try_into().unwrap())
    }
    assert_eq!(word(&mut data), 128);
    for case in 0..128 {
        let ps11 = word(&mut data) != 0;
        let count = word(&mut data);
        let mut frame = FramePlan {
            jobs: vec![],
            inserts: vec![],
        };
        for index in 0..count {
            let key = word(&mut data);
            let effect = word(&mut data) != 0;
            let light = word(&mut data);
            frame.jobs.push(if effect {
                Queued {
                    scene_job: index as usize,
                    sort_key: key,
                    kind: DrawKind::Effect,
                }
            } else {
                mesh(key, (light != u32::MAX).then_some(light as usize))
            });
        }
        let mut grab = [false; 256];
        for pass in 0..4 {
            if word(&mut data) != 0 {
                frame.inserts.push(FrameInsert {
                    scene_job: 0,
                    insert: InsertUse {
                        material_slot: 1,
                        render_pass: pass,
                    },
                });
            }
            grab[pass as usize] = word(&mut data) != 0;
        }
        let count = word(&mut data);
        let expected: Vec<_> = (0..count)
            .map(|_| {
                let action = word(&mut data);
                let arg = word(&mut data) as usize;
                match action {
                    0 => ClearScissor,
                    1 => ClearDestinationAlpha,
                    2 => AddDestinationAlpha,
                    3 => ResetShadowStates,
                    4 => ClearStencil,
                    5 => ApplyViewProjection,
                    6 => GrabCurrentRender,
                    7 => SetLight(arg),
                    8 => Draw(arg),
                    _ => panic!("invalid native action"),
                }
            })
            .collect();
        assert_eq!(actions(&frame, &grab, ps11), expected, "native case {case}");
    }
    assert!(data.is_empty());
}
