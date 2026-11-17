use std::{collections::HashMap, env, fs, path::PathBuf};

// Literal token: 0..127 followed by 1..128 bytes. Match token: high bit set,
// 3..130 bytes copied from a nonzero little-endian 16-bit backward distance.
fn pack(source: &[u8]) -> Vec<u8> {
    fn flush(literals: &mut Vec<u8>, output: &mut Vec<u8>) {
        if !literals.is_empty() {
            output.push((literals.len() - 1) as u8);
            output.extend_from_slice(literals);
            literals.clear();
        }
    }
    let mut previous = HashMap::<[u8; 4], usize>::new();
    let mut output = Vec::new();
    let mut literals = Vec::with_capacity(128);
    let mut position = 0;
    while position < source.len() {
        let key = source
            .get(position..position + 4)
            .map(|v| <[u8; 4]>::try_from(v).unwrap());
        let candidate = key.and_then(|key| previous.get(&key).copied());
        let mut length = 0;
        if let Some(candidate) = candidate.filter(|&candidate| position - candidate <= 65535) {
            while length < 130
                && position + length < source.len()
                && source[candidate + length] == source[position + length]
            {
                length += 1;
            }
            if length >= 4 {
                flush(&mut literals, &mut output);
                output.push(0x80 | (length - 3) as u8);
                output.extend_from_slice(&((position - candidate) as u16).to_le_bytes());
            }
        }
        if length < 4 {
            literals.push(source[position]);
            length = 1;
            if literals.len() == 128 {
                flush(&mut literals, &mut output);
            }
        }
        for index in position..position + length {
            if let Some(key) = source.get(index..index + 4) {
                previous.insert(key.try_into().unwrap(), index);
            }
        }
        position += length;
    }
    flush(&mut literals, &mut output);
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
