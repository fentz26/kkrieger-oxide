use kkrieger_oxide::{
    collision::Intersection,
    game::beta_monster::{
        subtractive::{StaticCell, StaticQuery, scan_static},
        world_cast::Ray,
    },
};
#[test]
fn static_scan_matches_complete_released_scan_without_dynamic_cells() {
    let bytes = include_bytes!("fixtures/beta-static-scan.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    let lists: [&[usize]; 4] = [&[4, 0, 1], &[2, 0], &[3, 4, 5], &[1, 5]];
    for case in 0..256 {
        let kind = words.next().unwrap();
        let kind = (kind != u32::MAX).then_some(kind);
        let monster_mode = words.next().unwrap() != 0;
        let mut interrupted = words.next().unwrap() != 0;
        let _first = words.next().unwrap() != 0;
        let direction: [f32; 4] = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let end = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let start = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let mut best = Intersection {
            plane: std::array::from_fn(|_| f32::from_bits(words.next().unwrap())),
            distance: f32::from_bits(words.next().unwrap()),
        };
        let mut modes = [0; 6];
        let mut flags = [false; 6];
        let mut planes = [[[0.; 4]; 6]; 6];
        for i in 0..6 {
            modes[i] = words.next().unwrap();
            flags[i] = words.next().unwrap() != 0;
            planes[i] = std::array::from_fn(|_| {
                std::array::from_fn(|_| f32::from_bits(words.next().unwrap()))
            });
        }
        let count = words.next().unwrap() as usize;
        let visited: [usize; 3] = std::array::from_fn(|_| words.next().unwrap() as usize);
        let cells: Vec<_> = (0..6)
            .map(|i| StaticCell {
                planes: &planes[i],
                mode: modes[i],
                only_for_shot: flags[i],
            })
            .collect();
        let query = StaticQuery {
            particle_kind: kind,
            monster_mode,
            ray: Ray {
                direction: direction[..3].try_into().unwrap(),
                end,
                start,
            },
        };
        let accepted = scan_static(
            &cells,
            &lists,
            &visited[..count],
            query,
            &mut best,
            &mut interrupted,
        )
        .unwrap();
        assert_eq!(
            u32::from(accepted),
            words.next().unwrap(),
            "case {case} accepted"
        );
        assert_eq!(
            u32::from(interrupted),
            words.next().unwrap(),
            "case {case} interrupt"
        );
        let actual = [
            best.plane[0].to_bits(),
            best.plane[1].to_bits(),
            best.plane[2].to_bits(),
            best.plane[3].to_bits(),
            best.distance.to_bits(),
        ];
        let expected: [u32; 5] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(actual, expected, "case {case} contact");
    }
    assert!(words.next().is_none());
}
#[test]
fn invalid_static_links_report_errors_after_entry_distance_reset() {
    let query = StaticQuery {
        particle_kind: None,
        monster_mode: false,
        ray: Ray {
            direction: [1., 0., 0.],
            end: [3., 0., 0., 1.],
            start: [-3., 0., 0., 1.],
        },
    };
    let mut best = Intersection {
        plane: [2., 3., 4., 5.],
        distance: -100.,
    };
    let mut interrupted = false;
    assert_eq!(
        scan_static(&[], &[], &[0], query, &mut best, &mut interrupted),
        Err("static additive list out of range")
    );
    assert_eq!(best.plane, [2., 3., 4., 5.]);
    assert_eq!(best.distance.to_bits(), 0x38d1b717);
    assert!(!interrupted);
    assert_eq!(
        scan_static(&[], &[&[0]], &[0], query, &mut best, &mut interrupted),
        Err("static subtractive cell out of range")
    );
}
