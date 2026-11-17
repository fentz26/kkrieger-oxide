//! Decode the original authored project once from its compact embedded stream.
use std::sync::OnceLock;
include!(concat!(env!("OUT_DIR"), "/embedded_size.rs"));
const PACKED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kkrieger.kxz"));
static PROJECT: OnceLock<Result<Box<[u8]>, &'static str>> = OnceLock::new();
pub fn data() -> Result<&'static [u8], &'static str> {
    PROJECT
        .get_or_init(|| unpack(PACKED, LENGTH))
        .as_deref()
        .map_err(|&error| error)
}
fn unpack(mut input: &[u8], length: usize) -> Result<Box<[u8]>, &'static str> {
    let mut output = Vec::with_capacity(length);
    while let Some((&token, rest)) = input.split_first() {
        input = rest;
        let count = if token & 0x80 == 0 {
            usize::from(token) + 1
        } else {
            usize::from(token & 0x7f) + 3
        };
        if count > length - output.len() {
            return Err("packed project exceeds declared size");
        }
        if token & 0x80 == 0 {
            let bytes = input.get(..count).ok_or("truncated project literal")?;
            output.extend_from_slice(bytes);
            input = &input[count..];
        } else {
            let bytes = input.get(..2).ok_or("truncated project match")?;
            input = &input[2..];
            let distance = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
            if distance == 0 || distance > output.len() {
                return Err("invalid project match distance");
            }
            for _ in 0..count {
                output.push(output[output.len() - distance]);
            }
        }
    }
    if output.len() != length {
        return Err("packed project is incomplete");
    }
    Ok(output.into_boxed_slice())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_overlap_and_input_bounds_are_checked() {
        assert_eq!(&*unpack(&[0, b'a', 0xff, 1, 0], 131).unwrap(), &[b'a'; 131]);
        for (data, size) in [
            (&[0][..], 1),
            (&[0x80][..], 3),
            (&[0x80, 0, 0][..], 3),
            (&[0x80, 1, 0][..], 3),
            (&[0, b'a'][..], 2),
            (&[0, b'a', 0, b'b'][..], 1),
        ] {
            assert!(unpack(data, size).is_err());
        }
        assert!(unpack(&[], 0).unwrap().is_empty());
        assert!(PACKED.len() < LENGTH / 2);
    }
}
