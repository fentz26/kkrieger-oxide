use kkrieger_oxide::game::beta_monster::{
    body::Particle,
    collision::{Completion, finish_particle},
};
fn cell(token: u32) -> Option<usize> {
    match token {
        0 => None,
        0x12345678 => Some(7),
        0x87654321 => Some(9),
        0xabcdef01 => Some(11),
        _ => panic!("unknown cell token"),
    }
}
fn token(cell: Option<usize>) -> u32 {
    match cell {
        None => 0,
        Some(7) => 0x12345678,
        Some(9) => 0x87654321,
        Some(11) => 0xabcdef01,
        _ => panic!("unknown cell index"),
    }
}
fn decode(raw: [u32; 32]) -> Particle {
    assert_eq!(&raw[8..10], &[0, 0]);
    Particle {
        flags: raw[..7].try_into().unwrap(),
        cell: cell(raw[7]),
        weight: f32::from_bits(raw[11]),
        damping: f32::from_bits(raw[10]),
        force: std::array::from_fn(|i| f32::from_bits(raw[12 + i])),
        vectors: std::array::from_fn(|v| {
            std::array::from_fn(|i| f32::from_bits(raw[16 + v * 4 + i]))
        }),
    }
}
fn encode(p: &Particle) -> [u32; 32] {
    let mut raw = [0; 32];
    raw[..7].copy_from_slice(&p.flags);
    raw[7] = token(p.cell);
    raw[10] = p.damping.to_bits();
    raw[11] = p.weight.to_bits();
    raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
    for (out, value) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
        *out = value.to_bits();
    }
    raw
}
#[test]
fn commit_and_rollback_match_released_zone_callback_order() {
    let bytes = include_bytes!("fixtures/beta-finish.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let input = std::array::from_fn(|_| words.next().unwrap());
        let target = std::array::from_fn(|_| f32::from_bits(words.next().unwrap()));
        let next_cell = cell(words.next().unwrap());
        let mode = words.next().unwrap() != 0;
        let completion = match words.next().unwrap() {
            0 => Completion::Clear,
            1 => Completion::Contact,
            2 => Completion::Blocked,
            _ => panic!("unknown completion"),
        };
        let changes: [u32; 17] = std::array::from_fn(|_| words.next().unwrap());
        let mut p = decode(input);
        let mut called = false;
        let mut args = [0; 10];
        let status = finish_particle(
            &mut p,
            target,
            next_cell,
            mode,
            completion,
            |p, target, old_cell, new_cell| {
                called = true;
                args[..4].copy_from_slice(&p.vectors[1].map(f32::to_bits));
                args[4..8].copy_from_slice(&target.map(f32::to_bits));
                args[8] = token(old_cell);
                args[9] = token(new_cell);
                p.vectors[1] = std::array::from_fn(|i| f32::from_bits(changes[i]));
                p.vectors[0] = std::array::from_fn(|i| f32::from_bits(changes[4 + i]));
                p.vectors[2] = std::array::from_fn(|i| f32::from_bits(changes[8 + i]));
                *target = std::array::from_fn(|i| f32::from_bits(changes[12 + i]));
                p.cell = cell(changes[16]);
            },
        );
        for (field, value) in encode(&p).into_iter().enumerate() {
            assert_eq!(value, words.next().unwrap(), "case {case} field {field}");
        }
        assert_eq!(status as u32, words.next().unwrap(), "case {case} status");
        assert_eq!(
            u32::from(called),
            words.next().unwrap(),
            "case {case} callback"
        );
        let expected: [u32; 10] = std::array::from_fn(|_| words.next().unwrap());
        assert_eq!(args, expected, "case {case} callback arguments");
    }
    assert!(words.next().is_none());
}
