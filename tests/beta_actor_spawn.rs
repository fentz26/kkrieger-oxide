use kkrieger_oxide::game::beta_monster::actor::Actor;
#[test]
fn authored_spawn_components_match_complete_released_prefix() {
    let bytes = include_bytes!("fixtures/beta-actor-spawn.bin");
    let mut words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b));
    assert_eq!(words.next(), Some(32));
    for case in 0..32 {
        let actor_token = words.next().unwrap();
        let present = words.next().unwrap() != 0;
        let input: [u32; 26] = std::array::from_fn(|_| words.next().unwrap());
        let parent =
            std::array::from_fn(|_| std::array::from_fn(|_| f32::from_bits(words.next().unwrap())));
        let mut calls = 0;
        let actor = Actor::spawn(&input, parent, 0, actor_token, |position| {
            calls += 1;
            assert_eq!(position[0], parent[3][0]);
            assert_eq!(position[2], parent[3][2]);
            present.then_some(0)
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actor.initial.life, 20);
        assert!(actor.hit_cell.monster);
        for (i, p) in actor.body.particles.iter().enumerate() {
            let expected: [u32; 32] = std::array::from_fn(|_| words.next().unwrap());
            let mut raw = [0; 32];
            raw[..7].copy_from_slice(&p.flags);
            raw[7] = if p.cell.is_some() { 0x3006000 } else { 0 };
            raw[8] = actor_token + 0x53c;
            raw[10] = p.damping.to_bits();
            raw[11] = p.weight.to_bits();
            raw[12..16].copy_from_slice(&p.force.map(f32::to_bits));
            for (out, v) in raw[16..].iter_mut().zip(p.vectors.into_iter().flatten()) {
                *out = v.to_bits();
            }
            assert_eq!(raw, expected, "case {case} particle {i}");
        }
        let count = words.next().unwrap() as usize;
        assert_eq!(actor.hit_cell.count, count);
        assert_eq!(actor.hit_cell.pose.count, count);
        for (field, v) in actor.hit_cell.pose.planes[..count]
            .iter()
            .flatten()
            .copied()
            .chain(actor.hit_cell.original[..count].iter().flatten().copied())
            .chain(actor.hit_cell.pose.center)
            .enumerate()
        {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} geometry field {field}"
            );
        }
        assert_eq!(actor.hit_cell.mode, words.next().unwrap());
        assert_eq!(
            u32::from(actor.hit_cell.pose.blocked),
            words.next().unwrap()
        );
        for (field, v) in actor
            .hit_cell
            .confirmed
            .into_iter()
            .flatten()
            .chain(actor.hit_cell.current.into_iter().flatten())
            .enumerate()
        {
            assert_eq!(
                v.to_bits(),
                words.next().unwrap(),
                "case {case} matrix field {field}"
            );
        }
    }
    assert!(words.next().is_none());
}
#[test]
fn truncated_spawn_fails_before_world_lookup() {
    assert!(
        Actor::spawn(
            &[0; 25],
            kkrieger_oxide::mesh::identity(),
            0,
            1,
            |_| panic!("unexpected lookup")
        )
        .is_err()
    );
}
