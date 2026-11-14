use kkrieger_oxide::{material::Program, mesh::Mesh, prepared_mesh::PreparedMesh, shadow::Context};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
}
#[test]
fn original_shadow_vertices_silhouettes_and_caps_match_cpp() {
    let mut r = Reader(include_bytes!("fixtures/shadow.bin"));
    assert_eq!(r.word(), 128);
    let mut context = Context::default();
    for c in 0..128 {
        let mut mesh = Mesh::cube(
            [1 + c % 3, 1 + (c / 3) % 3, 1 + (c / 9) % 3],
            (c % 16) as u32,
            [1., 1.5, 0.75, 0., 0., 0., -0.25, 0.125, 0.5],
        );
        for (i, f) in mesh.faces.iter_mut().enumerate() {
            f.material = ((i + c) % 3) as i32;
            f.element.used = (i + c) % 4 != 0;
        }
        mesh.materials.push(mesh.materials[1]);
        let mut programs = vec![vec![], vec![], vec![]];
        programs[1 + c % 2].push(Program::Shadow);
        let mesh = PreparedMesh::from_mesh(&mut mesh, &programs).unwrap();
        let light = std::array::from_fn(|_| r.float());
        let volume = context.build(&mesh, &mesh.jobs[0], light).unwrap();
        assert_eq!(r.word() as usize, volume.vertices.len());
        for v in &volume.vertices {
            for &value in v {
                assert_eq!(value.to_bits(), r.word());
            }
        }
        assert_eq!(r.word() as usize, volume.silhouette_indices, "case {c}");
        assert_eq!(r.word() as usize, volume.cap_indices, "case {c}");
        for &index in &volume.indices {
            assert_eq!(r.word() as usize, index, "case {c}");
        }
        assert_eq!(volume.draw_indices(false).len(), volume.silhouette_indices);
        assert_eq!(volume.draw_indices(true), volume.indices);
    }
    assert!(r.0.is_empty());
}
