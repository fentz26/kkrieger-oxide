use kkrieger_oxide::scene::light::beta_light_order;
#[test]
fn released_beta_light_sort_preserves_ties_and_clamps_count_after_sorting() {
    let mut bytes: &[u8] = include_bytes!("fixtures/beta-light-sort.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    assert_eq!(word(&mut bytes), 128);
    for case in 0..128 {
        let count = word(&mut bytes) as usize;
        let importance: Vec<_> = (0..count)
            .map(|_| f32::from_bits(word(&mut bytes)))
            .collect();
        let selected = beta_light_order(&importance);
        let count = word(&mut bytes) as usize;
        assert_eq!(selected.len(), count, "case {case}");
        for id in selected {
            assert_eq!(id, word(&mut bytes) as usize, "case {case}");
        }
    }
    assert!(bytes.is_empty());
}

#[test]
fn selected_light_records_retain_their_original_identity_and_geometry() {
    use kkrieger_oxide::{
        mesh::identity,
        scene::{
            culling::Frustum,
            light::{Prepared, beta_select},
        },
    };
    let candidates = [3., 2., 3., 4., 1.]
        .into_iter()
        .enumerate()
        .map(|(i, importance)| Prepared {
            operator: 100 + i,
            position: [i as f32, 2., 3.],
            flags: i as u32,
            color: 0x01020304 + i as u32,
            amplify: i as f32,
            range: 10. + i as f32,
            importance,
            rect: [i as f32, 0., 1., 1.],
            shadow: Frustum::from_view_projection(identity(), [-1., -1., 1., 1.]),
            zfail: Frustum::from_view_projection(identity(), [-1., -1., 1., 1.]),
        })
        .collect();
    let selected = beta_select(candidates);
    assert_eq!(
        selected.iter().map(|l| l.operator).collect::<Vec<_>>(),
        [103, 100, 102, 101]
    );
    for light in selected {
        let i = light.operator - 100;
        assert_eq!(light.position, [i as f32, 2., 3.]);
        assert_eq!(light.rect, [i as f32, 0., 1., 1.]);
        assert_eq!(light.flags, i as u32);
        assert_eq!(light.color, 0x01020304 + i as u32);
        assert_eq!(light.range, 10. + i as f32);
        assert_eq!(light.amplify, i as f32);
    }
}
