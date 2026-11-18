use kkrieger_oxide::{
    collision::Intersection,
    game::beta_monster::{
        subtractive::{
            CandidateQuery, CandidateWorld, DynamicCell, DynamicEffects, FirstParticle,
            PartBoxContact, ParticleContact, ParticleInfo, StaticCell,
        },
        world_cast::Ray,
    },
};
struct Effects {
    reflect: bool,
    trace: Vec<[u32; 16]>,
}
impl DynamicEffects for Effects {
    fn particle_hit(&mut self, hit: ParticleContact, select: &mut Option<i32>) {
        let mut record = [0; 16];
        record[..4].copy_from_slice(&[1, hit.cell as u32, hit.particle as u32, hit.kind]);
        record[4..8].copy_from_slice(&hit.position.map(f32::to_bits));
        record[8..12].copy_from_slice(&hit.plane.map(f32::to_bits));
        record[12] = u32::from(hit.monster);
        record[13] = 0;
        record[14] = select.map(|v| v as u32).unwrap_or(u32::MAX);
        record[15] = u32::from(select.is_some());
        self.trace.push(record);
        // Original Particle::Hit, with null hit-effect operator slots.
        if select.is_some() && (!self.reflect || hit.monster) {
            *select = Some(1);
        }
    }
    fn part_box_hit(&mut self, hit: PartBoxContact) {
        let mut record = [0; 16];
        record[..3].copy_from_slice(&[2, hit.cell as u32, hit.part_box as u32]);
        record[3..7].copy_from_slice(&hit.position.map(f32::to_bits));
        record[7..10].copy_from_slice(&hit.delta.map(f32::to_bits));
        record[10] = hit.mass.to_bits();
        self.trace.push(record);
    }
}
#[test]
fn full_candidate_scan_matches_native_dynamic_events_and_masks() {
    let bytes = include_bytes!("fixtures/beta-dynamic-scan.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    let static_planes = [
        [1., 0., 0., 2.5],
        [-1., 0., 0., -2.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let static_cells = [StaticCell {
        planes: &static_planes,
        mode: 1,
        only_for_shot: false,
    }];
    let lists: [&[usize]; 1] = [&[0]];
    for case in 0..256 {
        let direction: [f32; 4] = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let end = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let start = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let has_particle = words.next().unwrap() != 0;
        let kind = words.next().unwrap();
        let mass = f32::from_bits(words.next().unwrap());
        let has_first = words.next().unwrap() != 0;
        let id = words.next().unwrap() as usize;
        let first_kind = words.next().unwrap();
        let has_shot = words.next().unwrap() != 0;
        let select = words.next().unwrap() as i32;
        let reflect = words.next().unwrap() != 0;
        let monster_mode = words.next().unwrap() != 0;
        let mut interrupted = words.next().unwrap() != 0;
        let has_static = words.next().unwrap() != 0;
        let mut best = Intersection {
            plane: std::array::from_fn(|_| f32::from_bits(words.next().unwrap())),
            distance: f32::from_bits(words.next().unwrap()),
        };
        let mut modes = [0; 6];
        let mut only = [false; 6];
        let mut monsters = [false; 6];
        let mut players = [false; 6];
        let mut boxes = [None; 6];
        let mut masks = [0; 6];
        let mut planes = [[[0.; 4]; 6]; 6];
        for i in 0..6 {
            modes[i] = words.next().unwrap();
            only[i] = words.next().unwrap() != 0;
            monsters[i] = words.next().unwrap() != 0;
            players[i] = words.next().unwrap() != 0;
            let bx = words.next().unwrap();
            boxes[i] = (bx != u32::MAX).then_some(bx as usize);
            masks[i] = words.next().unwrap();
            planes[i] = std::array::from_fn(|_| {
                std::array::from_fn(|_| f32::from_bits(words.next().unwrap()))
            });
        }
        let mut cells: Vec<_> = (0..6)
            .map(|i| DynamicCell {
                planes: &planes[i],
                mode: modes[i],
                only_for_shot: only[i],
                monster: monsters[i],
                player: players[i],
                part_box: boxes[i],
                collided: masks[i],
            })
            .collect();
        let mut query = CandidateQuery {
            ray: Ray {
                direction: direction[..3].try_into().unwrap(),
                end,
                start,
            },
            particle: has_particle.then_some(ParticleInfo { kind, mass }),
            first: has_first.then_some(FirstParticle {
                id,
                kind: first_kind,
                shot_select: has_shot.then_some(select),
            }),
            monster_mode,
        };
        let mut effects = Effects {
            reflect,
            trace: Vec::new(),
        };
        let mut world = CandidateWorld {
            static_cells: &static_cells,
            lists: &lists,
            dynamic_cells: &mut cells,
        };
        let accepted = world
            .scan(
                if has_static { &[0] } else { &[] },
                &mut query,
                &mut best,
                &mut interrupted,
                &mut effects,
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
        let result_select = query.first.and_then(|first| first.shot_select);
        assert_eq!(
            u32::from(result_select.is_some()),
            words.next().unwrap(),
            "case {case} shot presence"
        );
        assert_eq!(
            result_select.unwrap_or(0) as u32,
            words.next().unwrap(),
            "case {case} select"
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
        for (i, cell) in cells.iter().enumerate() {
            assert_eq!(
                cell.collided,
                words.next().unwrap(),
                "case {case} cell {i} mask"
            );
        }
        let count = words.next().unwrap() as usize;
        assert_eq!(effects.trace.len(), count, "case {case} callbacks");
        for call in 0..12 {
            let expected: [u32; 16] = std::array::from_fn(|_| words.next().unwrap());
            assert_eq!(
                effects.trace.get(call).copied().unwrap_or([0; 16]),
                expected,
                "case {case} call {call}"
            );
        }
    }
    assert!(words.next().is_none());
}
