use kkrieger_oxide::{material::Program, mesh::Mesh, prepared_mesh::PreparedMesh};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn floats(&mut self, values: impl IntoIterator<Item = f32>) {
        for v in values {
            let e = f32::from_bits(self.word());
            assert!((v - e).abs() <= 2e-6, "{v} != {e}");
        }
    }
    fn words(&mut self, values: impl IntoIterator<Item = usize>) {
        for v in values {
            assert_eq!(v, self.word() as usize);
        }
    }
}
#[test]
fn engine_vertex_and_job_buffers_match_original_cpp() {
    let mut r = Reader(include_bytes!("fixtures/prepared-mesh.bin"));
    assert_eq!(r.word(), 64);
    for c in 0..64 {
        let mut mesh = Mesh::cube(
            [1 + c % 3, 1 + (c / 3) % 3, 1 + (c / 9) % 3],
            (c % 16) as u32,
            [1., 1.5, 0.75, 0., 0., 0., -0.25, 0.125, 0.5],
        );
        for (i, f) in mesh.faces.iter_mut().enumerate() {
            f.material = ((i + c) % 3) as i32;
            f.element.used = (i + c) % 4 != 0;
        }
        for (i, v) in mesh.vertices.iter_mut().enumerate() {
            v.attributes[3] = [
                (i % 7) as f32 * 0.25 - 0.25,
                (i % 5) as f32 * 0.375,
                (i % 3) as f32 * 0.5,
                0.75,
            ];
        }
        mesh.parts = vec![0];
        if c % 2 != 0 {
            mesh.parts.push(mesh.faces.len() / 2);
        }
        // A second source material slot is sufficient: preparation only needs programs.
        mesh.materials.push(mesh.materials[1]);
        let programs = vec![
            vec![],
            vec![
                Program::Static,
                Program::Shadow,
                Program::Sprites,
                Program::ThickLines,
            ],
            vec![
                Program::Static,
                Program::Shadow,
                Program::Sprites,
                Program::ThickLines,
            ],
        ];
        let prepared = PreparedMesh::from_mesh(&mut mesh, &programs).unwrap();
        assert_eq!(r.word() as usize, prepared.vertices.len());
        for v in &prepared.vertices {
            r.floats(v.position);
            r.floats(v.normal);
            r.floats(v.tangent);
            assert_eq!(r.word(), v.color);
            r.floats(v.uv);
        }
        assert_eq!(r.word() as usize, prepared.part_bounds.len());
        for b in &prepared.part_bounds {
            r.floats(*b);
        }
        assert_eq!(r.word() as usize, prepared.jobs.len());
        for j in &prepared.jobs {
            assert_eq!(r.word() as usize, j.material);
            assert_eq!(
                r.word(),
                match j.program {
                    Program::Static => 1,
                    Program::Shadow => 2,
                    Program::Sprites => 3,
                    Program::ThickLines => 4,
                }
            );
            r.floats(j.bounds.into_iter().flatten());
            assert_eq!(r.word() as usize, j.vertices.len());
            r.words(j.vertices.iter().copied());
            assert_eq!(r.word() as usize, j.indices.len());
            r.words(j.indices.iter().copied());
            assert_eq!(r.word() as usize, j.planes.len());
            for plane in &j.planes {
                r.floats(*plane);
            }
            assert_eq!(r.word() as usize, j.face_map.len());
            r.words(j.face_map.iter().copied());
            assert_eq!(r.word() as usize, j.edges.len());
            for e in &j.edges {
                r.words(e.vertices.map(usize::from));
                for face in e.faces {
                    assert_eq!(r.word() as i32, face);
                }
            }
            assert_eq!(r.word() as usize, j.face_part_end.len());
            for (f, e) in j.face_part_end.iter().zip(&j.edge_part_end) {
                assert_eq!(r.word(), f.map(|v| v as u32).unwrap_or(u32::MAX));
                assert_eq!(r.word(), e.map(|v| v as u32).unwrap_or(u32::MAX));
            }
        }
    }
    assert!(r.0.is_empty());
}

fn exported_assets(data: &[u8], layout: kkrieger_oxide::kx::Layout) {
    use kkrieger_oxide::{kx::Document, mesh_graph, prepared_mesh};
    let doc = Document::parse_layout(data, layout).unwrap();
    let mut roots = std::collections::BTreeSet::new();
    for op in &doc.operators {
        if mesh_graph::implemented(doc.classes[op.class].id) {
            continue;
        }
        for &input in op.inputs.iter().chain(op.links.iter().flatten()) {
            if mesh_graph::implemented(doc.classes[doc.operators[input].class].id) {
                roots.insert(input);
            }
        }
    }
    let roots: Vec<_> = roots.into_iter().collect();
    assert!(!roots.is_empty());
    let assets = prepared_mesh::generate_many(&doc, &roots).unwrap();
    assert_eq!(assets.meshes.len(), roots.len());
    let mut triangles = 0;
    for mesh in &assets.meshes {
        for job in &mesh.jobs {
            assert!(job.vertices.iter().all(|&i| i < mesh.vertices.len()));
            let scale = if job.program == Program::Shadow { 2 } else { 1 };
            assert!(job.indices.iter().all(|&i| i < job.vertices.len() * scale));
            assert_eq!(job.indices.len() % 3, 0);
            triangles += job.indices.len() / 3;
        }
    }
    assert!(triangles > 0 && !assets.materials.is_empty());
}
#[test]
#[ignore = "requires original named system fonts; full CPU preparation"]
fn development_assets_prepare() {
    exported_assets(
        include_bytes!("../assets/kkrieger3383.kx"),
        kkrieger_oxide::kx::Layout::Legacy3383,
    );
}
#[test]
#[ignore = "requires independently extracted original beta; full CPU preparation"]
fn beta_assets_prepare() {
    exported_assets(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        kkrieger_oxide::kx::Layout::Beta2004,
    );
}
