use kkrieger_oxide::kx::{Document, Layout};

const DATA: &[u8] = include_bytes!("../assets/kkrieger3383.kx");
fn fingerprint(doc: &Document<'_>) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    let mut add = |bytes: &[u8]| {
        for &b in bytes {
            hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
        }
    };
    for op in &doc.operators {
        add(&(doc.classes[op.class].id as u32).to_le_bytes());
        for &i in &op.inputs {
            add(&(i as u32).to_le_bytes());
        }
        for &i in &op.links {
            add(&i.map(|v| v as u32).unwrap_or(u32::MAX).to_le_bytes());
        }
        add(op.packed_parameters);
        add(op.animation);
        add(op.blob);
    }
    hash
}

#[test]
fn published_export_is_consumed_exactly_and_matches_independent_reader() {
    let d = Document::parse(DATA).unwrap();
    assert_eq!(d.layout, Layout::Legacy3383);
    assert_eq!(
        (
            d.operators.len(),
            d.classes.len(),
            d.events.len(),
            d.splines.len()
        ),
        (5174, 78, 2, 23)
    );
    assert_eq!(
        (d.song.len(), d.samples.len(), d.bpm),
        (27031, 6640, 89 * 65536)
    );
    assert_eq!(&d.roots[..3], &[Some(490), Some(650), Some(5173)]);
    assert!(d.roots[3..].iter().all(Option::is_none));
    assert_eq!(fingerprint(&d), 0x74c6b0051a7d0aa2);
    assert_eq!(
        d.operators.iter().filter(|op| op.animation != [1]).count(),
        113
    );
    let multiply = d.classes.iter().position(|c| c.id == 0x95).unwrap();
    assert_eq!(d.classes[multiply].data_words(), 17);
    assert_eq!(d.classes[multiply].packing.len(), 18);
    assert!(
        d.operators
            .iter()
            .filter(|o| o.class == multiply)
            .all(|o| o.parameters.len() == 18)
    );
    for (root, count) in [(490, 491), (650, 170), (5173, 4734)] {
        assert_eq!(d.reachable(root).unwrap().len(), count);
        let order = d.initialization_order(root).unwrap();
        assert_eq!(order.len(), count);
        let mut positions = vec![usize::MAX; d.operators.len()];
        for (p, &i) in order.iter().enumerate() {
            positions[i] = p;
        }
        for &i in &order {
            for &input in &d.operators[i].inputs {
                assert!(positions[input] < positions[i]);
            }
            if d.classes[d.operators[i].class].convention & 0x10000000 == 0 {
                for &link in d.operators[i].links.iter().flatten() {
                    assert!(positions[link] < positions[i]);
                }
            }
        }
    }
}

#[test]
fn truncated_and_mutated_data_never_panics() {
    for end in (0..DATA.len())
        .step_by(257)
        .chain(DATA.len() - 32..DATA.len())
    {
        assert!(
            Document::parse(&DATA[..end]).is_err(),
            "accepted truncation at {end}"
        );
    }
    let mut extra = DATA.to_vec();
    extra.push(0);
    assert!(Document::parse(&extra).is_err());
    let mut random = 0x12345678_u32;
    for size in 0..512 {
        let mut data = vec![0; size];
        for byte in &mut data {
            random ^= random << 13;
            random ^= random >> 17;
            random ^= random << 5;
            *byte = random as u8;
        }
        let _ = Document::parse(&data);
    }
}

fn minimal_current() -> Vec<u8> {
    let mut b = Vec::new();
    for w in [0_u32, 0, 89 * 65536, 1024 * 65536] {
        b.extend(w.to_le_bytes());
    }
    b.extend([1, 0]); // one operator, no splines
    b.push(0);
    b.extend([1; 15]); // root 0, remaining roots absent
    b.extend(0x02000000_u32.to_le_bytes()); // one class, no params or inputs
    b.extend(0x21_u16.to_le_bytes());
    b.push(0); // id and empty packing string
    b.extend(0_u32.to_le_bytes());
    b.extend([0, 1, 0]); // operator, animation END, zero events
    b
}

#[test]
fn modern_header_and_invalid_connections_are_checked() {
    let b = minimal_current();
    let d = Document::parse(&b).unwrap();
    assert_eq!(d.layout, Layout::Current);
    assert_eq!(d.roots[0], Some(0));
    let mut bad = b.clone();
    let len = bad.len();
    bad[len - 3] = 0x80; // shorthand input points before first node
    assert!(Document::parse(&bad).is_err());
    let mut bad = b.clone();
    bad[len - 3] = 127;
    assert!(Document::parse(&bad).is_err());
    let mut titled = b.clone();
    titled[..4].copy_from_slice(&3_u32.to_le_bytes());
    titled.splice(4..4, [b'X'; 32]);
    titled.splice(40..40, 0_u32.to_le_bytes()); // sample size follows zero-length song
    let d = Document::parse(&titled).unwrap();
    assert_eq!(d.title.unwrap(), [b'X'; 32]);
    assert_eq!(d.samples.len(), 0);
}

#[test]
#[ignore = "requires the independently extracted .reference/verified-beta.kx"]
fn original_beta_export_matches_independent_extraction() {
    let b = std::fs::read(".reference/verified-beta.kx").unwrap();
    let d = Document::parse_layout(&b, Layout::Beta2004).unwrap();
    assert_eq!(
        (
            b.len(),
            d.operators.len(),
            d.classes.len(),
            d.events.len(),
            d.splines.len()
        ),
        (121767, 4816, 77, 18, 38)
    );
    assert_eq!(fingerprint(&d), 0xad29100aa3b7a175);
    assert_eq!(d.reachable(4815).unwrap().len(), 4816);
    assert_eq!(d.initialization_order(4815).unwrap().len(), 4816);
}
