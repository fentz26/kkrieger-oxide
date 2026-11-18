use kkrieger_oxide::game::beta_monster::zones::{Transition, Zone, transition};
#[test]
fn transitions_match_released_zone_walk() {
    let bytes = include_bytes!("fixtures/beta-zones.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    let planes: Vec<_> = (0..3)
        .map(|i| {
            let x = i as f32 * 0.75;
            [
                [1., 0., 0., 1. - x],
                [-1., 0., 0., 1. + x],
                [0., 1., 0., 1.],
                [0., -1., 0., 1.],
                [0., 0., 1., 1.],
                [0., 0., -1., 1.],
            ]
        })
        .collect();
    for case in 0..256 {
        let kind = words.next().unwrap();
        let shot = words.next().unwrap() != 0;
        let select = words.next().unwrap() as i32;
        let same = words.next().unwrap() != 0;
        let n = words.next().unwrap() as usize;
        let a: [usize; 3] = std::array::from_fn(|_| words.next().unwrap() as usize);
        let m = words.next().unwrap() as usize;
        let b: [usize; 3] = std::array::from_fn(|_| words.next().unwrap() as usize);
        let start = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let end = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let mut zones: Vec<_> = (0..3)
            .map(|i| Zone {
                planes: &planes[i],
                enter: words.next().unwrap(),
                leave: words.next().unwrap(),
                collided: words.next().unwrap(),
            })
            .collect();
        let lists: [&[usize]; 2] = [&a[..n], &b[..m]];
        transition(
            &mut zones,
            &lists,
            Transition {
                old_cell: 0,
                new_cell: if same { 0 } else { 1 },
                start,
                end,
                kind,
                shot_select: shot.then_some(select),
            },
        )
        .unwrap();
        for (i, z) in zones.iter().enumerate() {
            for v in [z.enter, z.leave, z.collided] {
                assert_eq!(v, words.next().unwrap(), "case {case} zone {i}");
            }
        }
    }
    assert!(words.next().is_none());
}
