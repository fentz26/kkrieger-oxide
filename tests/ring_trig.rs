use kkrieger_oxide::mesh::Mesh;

#[test]
fn source_ring_keeps_double_trig_results_until_the_position_store() {
    let bytes = include_bytes!("fixtures/ring-trig.bin");
    assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 512);
    assert_eq!(bytes.len(), 4 + 512 * 16);
    for (case, row) in bytes[4..].as_chunks::<16>().0.iter().enumerate() {
        let words: [u32; 4] =
            std::array::from_fn(|i| u32::from_le_bytes(row[i * 4..i * 4 + 4].try_into().unwrap()));
        let mesh = Mesh::ring(2, 0, f32::from_bits(words[1]), f32::from_bits(words[0]), 0);
        let position = mesh.vertices[0].attributes[0];
        assert_eq!(position[0].to_bits(), words[2], "case {case} sine");
        assert_eq!(position[1].to_bits(), words[3], "case {case} cosine");
    }
}
