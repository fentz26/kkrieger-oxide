use kkrieger_oxide::game::beta_monster::body::Body;
#[test]
fn collider_translation_matches_native_positions() {
    let bytes = include_bytes!("fixtures/beta-body-translate.bin");
    let read = |b: &[u8], offset| f32::from_le_bytes(b[offset..offset + 4].try_into().unwrap());
    assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 128);
    assert_eq!(bytes.len(), 4 + 128 * 1296);
    for record in bytes[4..].as_chunks::<1296>().0 {
        let mut body = Body::new([0.; 4], None, 3);
        for (i, p) in body.particles.iter_mut().enumerate() {
            p.flags = std::array::from_fn(|a| read(record, i * 128 + a * 4).to_bits());
            p.cell = None;
            p.weight = read(record, i * 128 + 32);
            p.damping = read(record, i * 128 + 36);
            p.force = std::array::from_fn(|a| read(record, i * 128 + 48 + a * 4));
            p.vectors = std::array::from_fn(|v| {
                std::array::from_fn(|a| read(record, i * 128 + 64 + v * 16 + a * 4))
            });
        }
        let before = body.clone();
        body.translate(std::array::from_fn(|a| read(record, 640 + a * 4)));
        for (i, p) in body.particles.iter().enumerate() {
            assert_eq!(p.flags, before.particles[i].flags);
            assert_eq!(p.cell, before.particles[i].cell);
            assert_eq!(p.weight, before.particles[i].weight);
            assert_eq!(p.damping, before.particles[i].damping);
            assert_eq!(p.force, before.particles[i].force);
            assert_eq!(p.vectors[1..], before.particles[i].vectors[1..]);
            for (a, value) in p.vectors[0].iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    read(record, 656 + i * 128 + 64 + a * 4).to_bits()
                );
            }
        }
    }
}
