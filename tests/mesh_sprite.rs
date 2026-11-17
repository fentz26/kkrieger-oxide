use kkrieger_oxide::{
    material::Program,
    mesh_sprite,
    prepared_mesh::{Job, PreparedMesh, Vertex},
};
#[test]
fn original_sprite_positions_normals_uvs_and_batches_match_cpp() {
    let mut data = include_bytes!("fixtures/mesh-sprite.bin").as_slice();
    fn word(data: &mut &[u8]) -> u32 {
        let (v, rest) = data.split_at(4);
        *data = rest;
        u32::from_le_bytes(v.try_into().unwrap())
    }
    fn float(data: &mut &[u8]) -> f32 {
        f32::from_bits(word(data))
    }
    assert_eq!(word(&mut data), 64);
    for case in 0..64 {
        let count = word(&mut data) as usize;
        let size = float(&mut data);
        let matrix = std::array::from_fn(|_| std::array::from_fn(|_| float(&mut data)));
        let mesh = PreparedMesh {
            materials: vec![],
            vertices: (0..count)
                .map(|_| Vertex {
                    position: std::array::from_fn(|_| float(&mut data)),
                    normal: [0.; 3],
                    tangent: [0.; 3],
                    color: 0,
                    uv: [0.; 2],
                })
                .collect(),
            part_bounds: vec![],
            jobs: vec![],
            pass_jobs: vec![],
        };
        let job = Job {
            material: 0,
            program: Program::Sprites,
            bounds: [[0.; 3]; 2],
            vertices: (0..count).rev().collect(),
            indices: vec![],
            planes: vec![],
            face_map: vec![],
            edges: vec![],
            face_part_end: vec![],
            edge_part_end: vec![],
        };
        let batches = mesh_sprite::batches(&mesh, &job, size, matrix).unwrap();
        assert_eq!(batches.len(), word(&mut data) as usize, "case {case}");
        for batch in batches {
            assert_eq!(batch.len(), word(&mut data) as usize);
            for vertex in batch {
                for value in vertex
                    .position
                    .into_iter()
                    .chain(vertex.normal)
                    .chain(vertex.uv)
                {
                    assert_eq!(value.to_bits(), word(&mut data), "case {case}");
                }
            }
        }
    }
    assert!(data.is_empty());
}
