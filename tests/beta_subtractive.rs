use kkrieger_oxide::{
    collision::Intersection,
    game::beta_monster::{
        subtractive::{ClipOutcome, clip},
        world_cast::Ray,
    },
};
#[test]
fn subtractive_cell_clipping_matches_released_routine() {
    let bytes = include_bytes!("fixtures/beta-subtractive.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(512));
    for case in 0..512 {
        let count = words.next().unwrap() as usize;
        let mode = words.next().unwrap();
        let monster = words.next().unwrap() != 0;
        let interrupt = words.next().unwrap() != 0;
        let planes: [[f32; 4]; 6] =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let direction: [f32; 4] = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let end = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let start = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let mut best = Intersection {
            plane: std::array::from_fn(|_| f32::from_bits(words.next().unwrap())),
            distance: f32::from_bits(words.next().unwrap()),
        };
        let outcome = clip(
            &planes[..count],
            mode,
            monster,
            Ray {
                direction: direction[..3].try_into().unwrap(),
                end,
                start,
            },
            &mut best,
        );
        assert_eq!(
            u32::from(outcome != ClipOutcome::Miss),
            words.next().unwrap(),
            "case {case} accepted"
        );
        assert_eq!(
            u32::from(interrupt || outcome == ClipOutcome::Inside),
            words.next().unwrap(),
            "case {case} interrupt"
        );
        assert_eq!(
            u32::from(outcome == ClipOutcome::Inside),
            words.next().unwrap(),
            "case {case} inside"
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
