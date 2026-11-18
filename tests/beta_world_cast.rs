use kkrieger_oxide::{
    collision::Intersection,
    game::beta_monster::world_cast::{AdditiveCell, Ray, cast_additive},
};
fn box_planes(index: usize) -> [[f32; 4]; 6] {
    let x = 2. * index as f32;
    [
        [1., 0., 0., 1. - x],
        [-1., 0., 0., 1. + x],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ]
}
#[test]
fn additive_traversal_matches_released_hull_and_neighbor_walk() {
    let bytes = include_bytes!("fixtures/beta-world-cast.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(128));
    for case in 0..128 {
        let count = words.next().unwrap() as usize;
        let mut current = words.next().unwrap() as usize;
        let kind = words.next().unwrap();
        let kind = (kind != u32::MAX).then_some(kind);
        let _first = words.next().unwrap() != 0;
        let sub_hit = words.next().unwrap() != 0;
        let end = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let start = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let direction: [f32; 4] = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let plane = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let distance = f32::from_bits(words.next().unwrap());
        let mut links = Vec::new();
        let mut only = Vec::new();
        for _ in 0..count {
            only.push(words.next().unwrap() != 0);
            let n = words.next().unwrap() as usize;
            let slots: [u32; 4] = std::array::from_fn(|_| words.next().unwrap());
            links.push(slots[..n].iter().map(|&v| v as usize).collect::<Vec<_>>());
        }
        let planes: Vec<_> = (0..count).map(box_planes).collect();
        let cells: Vec<_> = (0..count)
            .map(|i| AdditiveCell {
                planes: &planes[i],
                neighbors: &links[i],
                only_for_shot: only[i],
            })
            .collect();
        let ray = Ray {
            end,
            start,
            direction: direction[..3].try_into().unwrap(),
        };
        let mut seen = Vec::new();
        let mut calls = 0;
        let result = cast_additive(&cells, &mut current, ray, kind, |visited| {
            calls += 1;
            seen.extend_from_slice(visited);
            sub_hit.then_some(Intersection { plane, distance })
        })
        .unwrap();
        assert_eq!(calls, 1, "case {case}");
        assert_eq!(current, words.next().unwrap() as usize, "case {case} cell");
        assert_eq!(
            result.is_some(),
            words.next().unwrap() != 0,
            "case {case} hit"
        );
        let output: [u32; 5] = std::array::from_fn(|_| words.next().unwrap());
        // The callback can write raw out buffers on false; the Option API
        // exposes contact fields only when the caller selects a hit.
        if let Some(hit) = result {
            let actual = [
                hit.plane[0].to_bits(),
                hit.plane[1].to_bits(),
                hit.plane[2].to_bits(),
                hit.plane[3].to_bits(),
                hit.distance.to_bits(),
            ];
            assert_eq!(actual, output, "case {case} contact");
        }
        let n = words.next().unwrap() as usize;
        let visited: [u32; 16] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(
            seen,
            visited[..n].iter().map(|&v| v as usize).collect::<Vec<_>>(),
            "case {case} visited"
        );
    }
    assert!(words.next().is_none());
}
#[test]
fn invalid_additive_links_fail_without_calling_subtracer() {
    let planes = box_planes(0);
    let links = [1];
    let cells = [AdditiveCell {
        planes: &planes,
        neighbors: &links,
        only_for_shot: false,
    }];
    let ray = Ray {
        start: [0., 0., 0., 1.],
        end: [3., 0., 0., 1.],
        direction: [1., 0., 0.],
    };
    let mut current = 1;
    assert_eq!(
        cast_additive(&cells, &mut current, ray, None, |_| panic!(
            "unexpected subtrace"
        )),
        Err("additive start cell out of range")
    );
    let mut current = 0;
    assert_eq!(
        cast_additive(&cells, &mut current, ray, None, |_| panic!(
            "unexpected subtrace"
        )),
        Err("additive neighbor out of range")
    );
    assert_eq!(current, 0);
}
