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
