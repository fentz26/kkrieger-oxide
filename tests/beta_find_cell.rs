use kkrieger_oxide::game::beta_monster::hull::find_cell;
#[test]
fn lookup_preserves_released_order_and_boundary_rules() {
    let bytes = include_bytes!("fixtures/beta-find-cell.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    let planes: Vec<_> = (0..6)
        .map(|i| {
            let x = i as f32 * 1.5;
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
        let count = words.next().unwrap() as usize;
        let order: [usize; 6] = std::array::from_fn(|_| words.next().unwrap() as usize);
        let point = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let expected = words.next().unwrap();
        let result = find_cell(
            order[..count].iter().map(|&i| (i, planes[i].as_slice())),
            point,
        );
        assert_eq!(
            result.map(|i| i as u32).unwrap_or(u32::MAX),
            expected,
            "case {case}"
        );
    }
    assert!(words.next().is_none());
}
