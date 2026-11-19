use kkrieger_oxide::mesh::{euler_matrix, euler_matrix_fsincos, srt_matrix};
#[test]
fn source_euler_helpers_store_x87_sine_and_cosine_as_single_precision() {
    let bytes = include_bytes!("fixtures/euler-fsincos.bin");
    assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 128);
    assert_eq!(bytes.len(), 4 + 128 * 12);
    for row in bytes[4..].as_chunks::<12>().0 {
        let words: [u32; 3] =
            std::array::from_fn(|i| u32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap()));
        let angle = f32::from_bits(words[0]);
        for matrix in [
            euler_matrix([angle, 0., 0.]),
            euler_matrix_fsincos([angle, 0., 0.]),
        ] {
            assert_eq!(matrix[1][2].to_bits(), words[1], "angle {angle}");
            assert_eq!(matrix[1][1].to_bits(), words[2], "angle {angle}");
        }
        // SRT accepts turns, then performs the same original float angle conversion.
        let turns = angle / std::f32::consts::TAU;
        let expected = euler_matrix_fsincos([turns * std::f32::consts::TAU, 0., 0.]);
        let matrix = srt_matrix(&[1., 1., 1., turns, 0., 0., 0., 0., 0.]);
        if turns.abs() >= 1e-6 {
            assert_eq!(matrix, expected);
        }
    }
}
