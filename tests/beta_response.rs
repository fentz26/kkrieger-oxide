use kkrieger_oxide::game::beta_monster::{
    body::Particle,
    collision::{Cast, Contact, ResponseWorld, respond},
};
fn cell(token: u32) -> Option<usize> {
    match token {
        0 => None,
        0x12345678 => Some(7),
        0x87654321 => Some(9),
        0x89abcdef => Some(10),
        0x9abcdef0 => Some(11),
        _ => panic!("unknown cell token"),
    }
}
fn token(cell: Option<usize>) -> u32 {
    match cell {
        None => 0,
        Some(7) => 0x12345678,
        Some(9) => 0x87654321,
        Some(10) => 0x89abcdef,
        Some(11) => 0x9abcdef0,
        _ => panic!("unknown cell index"),
    }
}
fn decode(raw: [u32; 32]) -> Particle {
    Particle {
        flags: raw[..7].try_into().unwrap(),
        cell: cell(raw[7]),
        damping: f32::from_bits(raw[10]),
        weight: f32::from_bits(raw[11]),
        force: std::array::from_fn(|i| f32::from_bits(raw[12 + i])),
        vectors: std::array::from_fn(|v| {
            std::array::from_fn(|i| f32::from_bits(raw[16 + v * 4 + i]))
        }),
    }
}
fn encode(p: &Particle, skip: bool, shot: bool) -> [u32; 32] {
    let mut raw = [0; 32];
    raw[..7].copy_from_slice(&p.flags);
    raw[7] = token(p.cell);
    raw[8] = if skip { 0x3001000 } else { 0 };
    raw[9] = if shot { 0x2468ace0 } else { 0 };
    raw[10] = p.damping.to_bits();
    raw[11] = p.weight.to_bits();
    raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
    for (out, value) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
        *out = value.to_bits();
    }
    raw
}
#[derive(Clone, Copy)]
struct Plan {
    hit: bool,
    interrupt: u32,
    cell: Option<usize>,
    plane: [f32; 4],
    distance: f32,
}
struct Boundary {
    skip: bool,
    shot: bool,
    flags: u32,
    interrupt: u32,
    plans: [Plan; 3],
    casts: usize,
    shot_changes: [f32; 5],
    zone_changes: [f32; 8],
    trace: Vec<[u32; 32]>,
}
impl Boundary {
    fn record(&mut self, words: impl IntoIterator<Item = u32>) {
        assert!(!self.skip || self.flags & 0x10 != 0);
        let mut record = [0; 32];
        for (i, value) in words.into_iter().enumerate() {
            record[i] = value;
        }
        self.trace.push(record);
    }
}
impl ResponseWorld for Boundary {
    fn skip_cell(&mut self, _: &Particle, skipped: bool) {
        if self.skip {
            if skipped {
                self.flags |= 0x10;
            } else {
                self.flags &= !0x10;
            }
        }
    }
    fn clear_interrupt(&mut self) {
        self.interrupt = 0;
    }
    fn interrupted(&self) -> bool {
        self.interrupt != 0
    }
    fn cast(&mut self, p: &mut Particle, q: &mut Cast, first: bool) -> Option<Contact> {
        self.record(
            [1, u32::from(first)]
                .into_iter()
                .chain(q.direction.map(f32::to_bits))
                .chain(q.target.map(f32::to_bits))
                .chain(q.origin.map(f32::to_bits))
                .chain([token(q.cell), self.flags, self.interrupt])
                .chain(p.vectors[0].map(f32::to_bits))
                .chain(p.vectors[2].map(f32::to_bits)),
        );
        let plan = self.plans[self.casts];
        self.casts += 1;
        q.cell = plan.cell;
        self.interrupt = plan.interrupt;
        plan.hit.then_some(Contact {
            plane: plan.plane,
            distance: plan.distance,
        })
    }
    fn shot(&mut self, _: &mut Particle, impact: &mut [f32; 4], plane: &mut [f32; 4]) {
        if !self.shot {
            return;
        }
        self.record(
            [2].into_iter()
                .chain(impact.map(f32::to_bits))
                .chain(plane.map(f32::to_bits))
                .chain([self.flags]),
        );
        *impact = self.shot_changes[..4].try_into().unwrap();
        plane[3] = self.shot_changes[4];
    }
    fn zones(
        &mut self,
        p: &mut Particle,
        target: &mut [f32; 4],
        old: Option<usize>,
        new: Option<usize>,
    ) {
        self.record(
            [3].into_iter()
                .chain(p.vectors[1].map(f32::to_bits))
                .chain(target.map(f32::to_bits))
                .chain([token(old), token(new), self.flags])
                .chain(p.vectors[0].map(f32::to_bits))
                .chain(p.vectors[2].map(f32::to_bits)),
        );
        p.vectors[1] = self.zone_changes[..4].try_into().unwrap();
        *target = self.zone_changes[4..].try_into().unwrap();
    }
}
#[test]
fn complete_response_matches_released_cast_and_callback_trace() {
    let bytes = include_bytes!("fixtures/beta-response.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(256));
    for case in 0..256 {
        let raw: [u32; 32] = std::array::from_fn(|_| words.next().unwrap());
        let mode = words.next().unwrap() != 0;
        let plans = std::array::from_fn(|_| Plan {
            hit: words.next().unwrap() != 0,
            interrupt: words.next().unwrap(),
            cell: cell(words.next().unwrap()),
            plane: std::array::from_fn(|_| f32::from_bits(words.next().unwrap())),
            distance: f32::from_bits(words.next().unwrap()),
        });
        let mut world = Boundary {
            skip: raw[8] != 0,
            shot: raw[9] != 0,
            flags: words.next().unwrap(),
            interrupt: words.next().unwrap(),
            plans,
            casts: 0,
            shot_changes: std::array::from_fn(|_| f32::from_bits(words.next().unwrap())),
            zone_changes: std::array::from_fn(|_| f32::from_bits(words.next().unwrap())),
            trace: Vec::new(),
        };
        let mut p = decode(raw);
        let result = respond(&mut p, mode, &mut world);
        for (field, value) in encode(&p, world.skip, world.shot).into_iter().enumerate() {
            assert_eq!(value, words.next().unwrap(), "case {case} field {field}");
        }
        assert_eq!(result as u32, words.next().unwrap(), "case {case} status");
        assert_eq!(
            world.flags,
            words.next().unwrap(),
            "case {case} skip cleanup"
        );
        assert_eq!(
            world.interrupt,
            words.next().unwrap(),
            "case {case} interrupt"
        );
        let count = words.next().unwrap() as usize;
        assert_eq!(world.trace.len(), count, "case {case} trace count");
        for call in 0..5 {
            let expected: [u32; 32] = std::array::from_fn(|_| words.next().unwrap());
            assert_eq!(
                world.trace.get(call).copied().unwrap_or([0; 32]),
                expected,
                "case {case} call {call}"
            );
        }
    }
    assert!(words.next().is_none());
}
