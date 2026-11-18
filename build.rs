use std::{collections::HashMap, env, fs, path::PathBuf};

// Literal token: 0..127 followed by 1..128 bytes. Match token: high bit set,
// 3..130 bytes copied from a nonzero little-endian 16-bit backward distance.
pub(crate) fn pack(source: &[u8]) -> Vec<u8> {
    // Search prior equal three-byte prefixes, retaining all legal backward
    // distances. Build-time work does not increase decoder or executable size.
    let mut heads = HashMap::<[u8; 3], usize>::new();
    let mut previous = vec![usize::MAX; source.len()];
    let mut matches = vec![(0usize, 0usize); source.len()];
    for position in 0..source.len() {
        let Some(key) = source.get(position..position + 3) else {
            continue;
        };
        let key: [u8; 3] = key.try_into().unwrap();
        let mut candidate = heads.insert(key, position).unwrap_or(usize::MAX);
        previous[position] = candidate;
        let limit = 130.min(source.len() - position);
        while candidate != usize::MAX && position - candidate <= 65535 {
            let mut length = 3;
            while length < limit && source[candidate + length] == source[position + length] {
                length += 1;
            }
            if length > matches[position].0 {
                matches[position] = (length, position - candidate);
                if length == limit {
                    break;
                }
            }
            candidate = previous[candidate];
        }
    }
    // Choose the cheapest complete parse, rather than greedily committing to
    // the longest match. All prefixes of the longest match share its distance.
    let mut costs = vec![0usize; source.len() + 1];
    let mut choices = vec![(0usize, 0usize); source.len()];
    for position in (0..source.len()).rev() {
        let mut best = usize::MAX;
        for length in 1..=128.min(source.len() - position) {
            let cost = 1 + length + costs[position + length];
            if cost < best {
                best = cost;
                choices[position] = (length, 0);
            }
        }
        let (maximum, distance) = matches[position];
        for length in 3..=maximum {
            let cost = 3 + costs[position + length];
            if cost < best {
                best = cost;
                choices[position] = (length, distance);
            }
        }
        costs[position] = best;
    }
    let mut output = Vec::with_capacity(costs[0]);
    let mut position = 0;
    while position < source.len() {
        let (length, distance) = choices[position];
        if distance == 0 {
            output.push((length - 1) as u8);
            output.extend_from_slice(&source[position..position + length]);
        } else {
            output.push(0x80 | (length - 3) as u8);
            output.extend_from_slice(&(distance as u16).to_le_bytes());
        }
        position += length;
    }
    output
}

fn main() {
    println!("cargo:rerun-if-changed=assets/kkrieger3383.kx");
    let source = fs::read("assets/kkrieger3383.kx").expect("read authored project");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(output.join("kkrieger.kxz"), pack(&source)).expect("write packed project");
    fs::write(
        output.join("embedded_size.rs"),
        format!("const LENGTH: usize = {};\n", source.len()),
    )
    .expect("write project size");
}
