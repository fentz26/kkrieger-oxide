use kkrieger_oxide::mesh::{Element, Mesh, Selection};

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let (word, rest) = self.0.split_at(4);
        self.0 = rest;
        u32::from_le_bytes(word.try_into().unwrap())
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn element(&mut self, e: Element) {
        assert_eq!(
            [e.mask as u32, e.id as u32, e.selected as u32, e.used as u32],
            std::array::from_fn::<_, 4, _>(|_| self.word())
        );
    }
    fn integers(&mut self, values: impl IntoIterator<Item = i32>) {
        for v in values {
            assert_eq!(v, self.word() as i32);
        }
    }
}

#[test]
fn ring_and_extrusion_storage_match_original_cpp() {
    let mut r = Reader(include_bytes!("fixtures/mesh-ring.bin"));
    assert_eq!(r.word(), 256);
    for case in 0..256 {
        let segments = r.word() as usize;
        let mask = r.word();
        let radius = r.float();
        let phase = r.float();
        let arc = r.word() as usize;
        let mut m = Mesh::ring(segments, mask, radius, phase, arc);
        if r.word() != 0 {
            for (i, e) in m.edges.iter_mut().enumerate() {
                e.crease = if i % 2 == 1 { 6 } else { 16 };
            }
        }
        let stages = r.word();
        for _ in 0..stages {
            for (i, f) in m.faces.iter_mut().enumerate() {
                f.element.selected = i == 0;
            }
            let selection = [
                Selection::Add,
                Selection::Subtract,
                Selection::Set,
                Selection::SetNot,
            ][case % 4];
            m.extrude(mask, selection, (case % 2) as i32);
        }

        verify_mesh(&mut r, &m, case);
    }
    assert!(r.0.is_empty());
}

fn verify_mesh(r: &mut Reader, m: &Mesh, case: usize) {
    assert_eq!(
        [m.vertices.len(), m.edges.len(), m.faces.len()],
        std::array::from_fn::<_, 3, _>(|_| r.word() as usize)
    );
    for (vertex_index, v) in m.vertices.iter().enumerate() {
        let e = v.element;
        assert_eq!(
            [e.mask as u32, e.id as u32, e.selected as u32, e.used as u32],
            std::array::from_fn::<_, 4, _>(|_| r.word()),
            "root {case} vertex {vertex_index} element"
        );
        r.integers([v.next, v.first, v.temp, v.temp2, v.reindex]);
        r.integers(v.weight.map(i32::from));
        r.integers(v.matrix.map(i32::from));
        for &actual in v.attributes.iter().flatten() {
            let expected = r.float();
            assert!(
                (actual - expected).abs() <= 2e-6,
                "case {case}: {actual} != {expected}"
            );
        }
    }
    for e in &m.edges {
        r.element(e.element);
        for a in [e.next, e.previous, e.face, e.vertex, e.temp] {
            r.integers(a);
        }
        r.integers([e.crease]);
    }
    for f in &m.faces {
        r.element(f.element);
        r.integers([f.material, f.edge, f.temp, f.temp2, f.temp3]);
    }
    // Closed face and vertex cycles are required by subsequent topology operations.
    for h in 0..2 * m.edges.len() {
        assert_eq!(m.previous_face_edge(m.next_face_edge(h)), h);
        assert_eq!(m.next_face_edge(m.previous_face_edge(h)), h);
        assert_eq!(m.previous_vertex_edge(m.next_vertex_edge(h)), h);
        assert_eq!(
            m.vertices[m.vertex_id(m.next_vertex_edge(h))].first,
            m.vertices[m.vertex_id(h)].first
        );
        assert_eq!(m.face_id(m.next_face_edge(h)), m.face_id(h));
    }
}

#[test]
fn original_cube_tessellation_uvs_and_collision_match_cpp() {
    verify_cubes(include_bytes!("fixtures/mesh-cube.bin"), 128, None, false);
}

fn verify_cubes(
    bytes: &[u8],
    count: usize,
    document: Option<&kkrieger_oxide::kx::Document>,
    normals: bool,
) {
    let mut r = Reader(bytes);
    assert_eq!(r.word() as usize, count);
    let operators: Vec<_> = document
        .map(|d| {
            d.operators
                .iter()
                .filter(|o| d.classes[o.class].id == 0x81)
                .collect()
        })
        .unwrap_or_default();
    if document.is_some() {
        assert_eq!(operators.len(), count);
    }
    for case in 0..count {
        let tessellation = std::array::from_fn(|_| r.word() as usize);
        let flags = r.word();
        let srt = std::array::from_fn(|_| r.float());
        if let Some(op) = operators.get(case) {
            assert_eq!(
                op.parameters[..4],
                [
                    tessellation[0] as u32,
                    tessellation[1] as u32,
                    tessellation[2] as u32,
                    flags
                ]
            );
            for (expected, &bits) in srt.iter().zip(&op.parameters[4..13]) {
                assert_eq!(expected.to_bits(), bits);
            }
        }
        let mut m = Mesh::cube(tessellation, flags, srt);
        if normals {
            for (i, f) in m.faces.iter_mut().enumerate() {
                f.element.selected = (i + case) % 3 != 0;
            }
            m.calculate_normals(case % 2 != 0, 1 + (case % 7) as u32);
        }
        verify_mesh(&mut r, &m, case);
        assert_eq!(m.collision.len(), r.word() as usize);
        for c in &m.collision {
            r.integers(c.vertices.map(|v| v as i32));
            r.integers([
                c.mode,
                c.code as i32,
                c.condition as i32,
                c.value,
                c.output as i32,
            ]);
            assert_eq!(c.events, [None; 2]);
        }
        assert_eq!(m.parts.len(), r.word() as usize);
        r.integers(m.parts.iter().map(|&v| v as i32));
    }
    assert!(r.0.is_empty());
}

#[test]
fn every_development_cube_matches_original_cpp() {
    use kkrieger_oxide::kx::Document;
    let document = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    verify_cubes(
        include_bytes!("fixtures/mesh-cube3383.bin"),
        224,
        Some(&document),
        false,
    );
}

#[test]
#[ignore = "requires local original beta data and C++ mesh oracle"]
fn every_beta_cube_matches_original_cpp() {
    use kkrieger_oxide::kx::{Document, Layout};
    let data = std::fs::read(".reference/verified-beta.kx").unwrap();
    let document = Document::parse_layout(&data, Layout::Beta2004).unwrap();
    verify_cubes(
        &std::fs::read(".reference/mesh-cube-beta.bin").unwrap(),
        224,
        Some(&document),
        false,
    );
}

#[test]
fn original_crease_aware_normals_and_tangents_match_cpp() {
    verify_cubes(
        include_bytes!("fixtures/mesh-cube-normals.bin"),
        128,
        None,
        true,
    );
}

fn verify_cylinders(
    bytes: &[u8],
    expected_count: usize,
    document: Option<&kkrieger_oxide::kx::Document>,
) {
    let mut r = Reader(bytes);
    assert_eq!(r.word() as usize, expected_count);
    let operators: Vec<_> = document
        .map(|d| {
            d.operators
                .iter()
                .filter(|o| d.classes[o.class].id == 0x82)
                .collect()
        })
        .unwrap_or_default();
    if document.is_some() {
        assert_eq!(operators.len(), expected_count);
    }
    for case in 0..expected_count {
        let parameters = std::array::from_fn::<_, 5, _>(|_| r.word());
        if document.is_some() {
            let mut expected = [0; 5];
            expected[..operators[case].parameters.len()]
                .copy_from_slice(&operators[case].parameters);
            assert_eq!(expected, parameters);
        }
        let m = Mesh::cylinder(
            parameters[0] as usize,
            parameters[1] as usize,
            parameters[2],
            parameters[3] as usize,
            parameters[4] as usize,
        );
        verify_mesh(&mut r, &m, case);
        assert!(m.collision.is_empty());
        assert_eq!(m.parts.len(), r.word() as usize);
        r.integers(m.parts.iter().map(|&v| v as i32));
    }
    assert!(r.0.is_empty());
}

#[test]
fn cylinder_caps_arcs_rings_and_uvs_match_original_cpp() {
    verify_cylinders(include_bytes!("fixtures/mesh-cylinder.bin"), 192, None);
}

#[test]
fn every_development_cylinder_matches_original_cpp() {
    let document =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    verify_cylinders(
        include_bytes!("fixtures/mesh-cylinder3383.bin"),
        41,
        Some(&document),
    );
}

#[test]
#[ignore = "requires local original beta data and C++ cylinder oracle"]
fn every_beta_cylinder_matches_original_cpp() {
    use kkrieger_oxide::kx::{Document, Layout};
    let data = std::fs::read(".reference/verified-beta.kx").unwrap();
    let document = Document::parse_layout(&data, Layout::Beta2004).unwrap();
    verify_cylinders(
        &std::fs::read(".reference/mesh-cylinder-beta.bin").unwrap(),
        39,
        Some(&document),
    );
}

#[test]
fn selected_face_subdivision_and_attribute_creases_match_cpp() {
    let mut r = Reader(include_bytes!("fixtures/mesh-subdivision.bin"));
    assert_eq!(r.word(), 48);
    for case in 0..48 {
        let tessellation = std::array::from_fn(|_| r.word() as usize);
        let flags = r.word();
        let alpha = r.float();
        let count = r.word();
        let mut m = Mesh::cube(tessellation, flags, [1., 1., 1., 0., 0., 0., 0., 0., 0.]);
        for (i, f) in m.faces.iter_mut().enumerate() {
            f.element.selected = case % 3 == 0 || (i + case) % 3 != 0;
        }
        for _ in 0..count {
            m.subdivide(alpha);
        }
        verify_mesh(&mut r, &m, case);
        assert_eq!(m.parts.len(), r.word() as usize);
        r.integers(m.parts.iter().map(|&v| v as i32));
    }
    assert!(r.0.is_empty());
}

#[test]
fn assembly_collision_logic_materials_and_indices_match_original_cpp() {
    use kkrieger_oxide::mesh::{Material, MaterialSource};
    let mut r = Reader(include_bytes!("fixtures/mesh-assembly.bin"));
    assert_eq!(r.word(), 48);
    for case in 0..48 {
        let c = case as i32;
        let mut m = Mesh::default();
        m.append(&Mesh::collision_cube(
            [-1.25, 0.75, -2., 1.5, -0.5, 2.25],
            c % 32,
            [1 + case % 3, 1 + (case / 3) % 2, 1 + (case / 6) % 4],
            [257 + c, 511 - c, -71 * c, 769 + c],
            [Some(37), Some(89)],
        ));
        let mut other = Mesh::cube(
            [1 + case % 2, 1, 1 + (case / 2) % 2],
            (case % 16) as u32,
            [1., 1., 1., 0., 0., 0., 0., 0., 0.],
        );
        other.materials.push(Material {
            source: MaterialSource::Operator(29),
            pass: c % 3,
        });
        other.materials.push(Material {
            source: MaterialSource::Operator(45),
            pass: c % 2,
        });
        for (i, f) in other.faces.iter_mut().enumerate() {
            f.material = (i % 4) as i32;
        }
        other.light_slots.push(other.vertices.len() - 1);
        other.pivot = Some(2);
        for _ in 0..1 + case % 3 {
            m.append(&other);
        }
        verify_mesh(&mut r, &m, case);
        assert_eq!(m.collision.len(), r.word() as usize);
        for cell in &m.collision {
            r.integers(cell.vertices.map(|v| v as i32));
            r.integers([
                cell.mode,
                cell.code as i32,
                cell.condition as i32,
                cell.value,
                cell.output as i32,
            ]);
            for event in cell.events {
                assert_eq!(event.map(|id| id as u32).unwrap_or(u32::MAX), r.word());
            }
        }
        assert_eq!(m.materials.len(), r.word() as usize);
        for material in &m.materials {
            let id = match material.source {
                MaterialSource::Null => 0,
                MaterialSource::Default => 1,
                MaterialSource::Operator(id) => id as u32 + 2,
            };
            assert_eq!(id, r.word());
            assert_eq!(material.pass, r.word() as i32);
        }
        assert_eq!(m.light_slots.len(), r.word() as usize);
        r.integers(m.light_slots.iter().map(|&v| v as i32));
        assert_eq!(m.parts.len(), r.word() as usize);
        r.integers(m.parts.iter().map(|&v| v as i32));
        assert_eq!(m.pivot.map(|v| v as i32).unwrap_or(-1), r.word() as i32);
    }
    assert!(r.0.is_empty());
}

fn verify_composed_graph(
    data: &[u8],
    layout: kkrieger_oxide::kx::Layout,
    reference: &[u8],
    expected_count: usize,
) {
    use kkrieger_oxide::{kx::Document, mesh::MaterialSource, mesh_graph};
    let document = Document::parse_layout(data, layout).unwrap();
    let mut scan = Reader(reference);
    assert_eq!(scan.word() as usize, expected_count);
    let mut roots = Vec::new();
    for _ in 0..expected_count {
        roots.push(scan.word() as usize);
        let nv = scan.word() as usize;
        let ne = scan.word() as usize;
        let nf = scan.word() as usize;
        scan.0 = &scan.0[(nv * 37 + ne * 15 + nf * 9) * 4..];
        for stride in [15, 2, 1, 1] {
            let count = scan.word() as usize;
            scan.0 = &scan.0[count * stride * 4..];
        }
        scan.0 = &scan.0[8..];
    }
    assert!(scan.0.is_empty());
    let meshes = mesh_graph::generate_many(&document, &roots).unwrap();
    let mut r = Reader(reference);
    r.word();
    for (&root, m) in roots.iter().zip(meshes) {
        assert_eq!(root, r.word() as usize);
        verify_mesh(&mut r, &m, root);
        assert_eq!(
            m.collision.len(),
            r.word() as usize,
            "op {root} collision count"
        );
        for cell in &m.collision {
            r.integers(cell.vertices.map(|v| v as i32));
            r.integers([
                cell.mode,
                cell.code as i32,
                cell.condition as i32,
                cell.value,
                cell.output as i32,
            ]);
            for event in cell.events {
                assert_eq!(event.map(|id| id as u32).unwrap_or(u32::MAX), r.word());
            }
        }
        assert_eq!(m.materials.len(), r.word() as usize);
        for material in &m.materials {
            let id = match material.source {
                MaterialSource::Null => 0,
                MaterialSource::Default => 1,
                MaterialSource::Operator(id) => id as u32 + 2,
            };
            assert_eq!(id, r.word(), "op {root} material identity");
            assert_eq!(material.pass, r.word() as i32);
        }
        assert_eq!(m.light_slots.len(), r.word() as usize);
        r.integers(m.light_slots.iter().map(|&v| v as i32));
        assert_eq!(m.parts.len(), r.word() as usize);
        r.integers(m.parts.iter().map(|&v| v as i32));
        assert_eq!(m.pivot.map(|v| v as i32).unwrap_or(-1), r.word() as i32);
        assert_eq!(m.got_normals, r.word() != 0, "op {root} normal cache state");
    }
    assert!(r.0.is_empty());
}

fn unpack_fixture(bytes: &[u8]) -> Vec<u8> {
    assert_eq!(&bytes[..4], b"WZ01");
    let expected = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let mut input = Reader(&bytes[8..]);
    let mut output = Vec::with_capacity(expected);
    while !input.0.is_empty() {
        let token = input.word();
        let length = (token & 0x7fffffff) as usize;
        assert!(length > 0 && length <= (expected - output.len()) / 4);
        if token >> 31 != 0 {
            let distance = input.word() as usize;
            assert!(distance > 0 && distance <= output.len() / 4);
            for _ in 0..length {
                let start = output.len() - distance * 4;
                let word = std::array::from_fn::<_, 4, _>(|i| output[start + i]);
                output.extend_from_slice(&word);
            }
        } else {
            output.extend_from_slice(&input.0[..length * 4]);
            input.0 = &input.0[length * 4..];
        }
    }
    assert_eq!(output.len(), expected);
    output
}

#[test]
fn development_composed_geometry_matches_original_cpp() {
    let reference = unpack_fixture(include_bytes!("fixtures/mesh-graph3383.wz"));
    verify_composed_graph(
        include_bytes!("../assets/kkrieger3383.kx"),
        kkrieger_oxide::kx::Layout::Legacy3383,
        &reference,
        180,
    );
}

#[test]
#[ignore = "requires local original beta data and composed geometry oracle"]
fn beta_composed_geometry_matches_original_cpp() {
    verify_composed_graph(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        kkrieger_oxide::kx::Layout::Beta2004,
        &std::fs::read(".reference/mesh-graph-beta.bin").unwrap(),
        171,
    );
}

#[test]
fn original_bending_bevel_and_texture_displacement_match_cpp() {
    let mut r = Reader(include_bytes!("fixtures/mesh-deform.bin"));
    assert_eq!(r.word(), 64);
    let mut bitmap = kkrieger_oxide::bitmap::Bitmap::flat(5, 4, 0, 0);
    for y in 0..16 {
        for x in 0..32 {
            for (channel, value) in bitmap.pixels[y * 32 + x].iter_mut().enumerate() {
                *value = ((x * 113 + y * 349 + channel * 997) % 32768) as u16;
            }
        }
    }
    for case in 0..64 {
        let c = case as i32;
        let mut m = Mesh::cube(
            [1 + case % 2, 2 + (case / 2) % 2, 1],
            (case % 2) as u32,
            [1., 1., 1., 0., 0., 0., 0., 0., 0.],
        );
        for (i, v) in m.vertices.iter_mut().enumerate() {
            v.element.mask = u8::from(i % 3 != 0);
        }
        if case % 2 == 0 {
            m.all_to_selection(true, 7);
        } else {
            m.mask_to_selection(1 << 16);
        }
        m.bend_between(
            [
                1. + (case % 3) as f32 * 0.125,
                1.,
                0.75,
                0.015625,
                -0.03125,
                0.,
                0.,
                -0.125,
                0.25,
            ],
            [0.5, 1.25, 1., -0.0625, 0., 0.03125, 0.25, 0., -0.125],
            [0.0625, -0.03125],
            [-0.75, 0.625],
            (case % 8) as u32,
        );
        m.all_to_selection(true, 7);
        m.bend_curve(
            [0.125, -0.25, 0.0625],
            [0.03125, -0.0625, 0.015625],
            0.25 + (case % 3) as f32 * 0.25,
            (c % 7 - 3) as f32 * 0.0625,
        );
        m.need_all_normals();
        m.all_to_selection(false, 7);
        m.faces[0].element.selected = true;
        m.bevel(
            (case % 3) as f32 * 0.05,
            (c % 4 - 1) as f32 * 0.03125,
            (case % 2) as i32,
            0,
            Selection::Add,
        );
        m.displace(&bitmap, [0.125, -0.25, 0.375]);
        verify_mesh(&mut r, &m, case);
        assert_eq!(m.got_normals, r.word() != 0);
    }
    assert!(r.0.is_empty());
}

#[test]
fn mesh_perlin_matches_original_x87_directed_single_precision() {
    let mut r = Reader(include_bytes!("fixtures/mesh-perlin3.bin"));
    assert_eq!(r.word(), 128);
    let noise = kkrieger_oxide::random::Noise::new();
    for case in 0..128 {
        let matrix = std::array::from_fn(|_| std::array::from_fn(|_| r.float()));
        let amplitude = std::array::from_fn(|_| r.float());
        let count = r.word() as usize;
        let mut mesh = Mesh::default();
        let mut expected = Vec::new();
        for _ in 0..count {
            let point = std::array::from_fn(|_| r.float());
            let selected = r.word() != 0;
            let vertex = mesh.add_vertex();
            mesh.vertices[vertex].attributes[0] = point;
            mesh.vertices[vertex].element.selected = selected;
            expected.push(std::array::from_fn::<_, 4, _>(|_| r.word()));
        }
        mesh.perlin(&matrix, amplitude, &noise);
        for (i, (vertex, expected)) in mesh.vertices.iter().zip(expected).enumerate() {
            assert_eq!(
                vertex.attributes[0].map(f32::to_bits),
                expected,
                "case {case} vertex {i}"
            );
        }
    }
    assert!(r.0.is_empty());
}

#[test]
fn original_multiplication_random_uv_local_rotation_and_extrusion_match_cpp() {
    use kkrieger_oxide::mesh::MaterialSource;
    let mut r = Reader(include_bytes!("fixtures/mesh-multiply.bin"));
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let c = case as i32;
        let mut input = Mesh::cube(
            [1 + case % 2, 1 + (case / 2) % 2, 1],
            ((case % 16) | (((case / 16) % 4) * 16)) as u32,
            [1., 1., 1., 0., 0., 0., 0., 0., 0.],
        );
        let m = input.multiplied(
            [
                1.03125, 0.875, 1.125, 0.0625, -0.03125, 0.125, 0., 0.25, -0.125,
            ],
            c % 7 - 1,
            (case % 4) as u32,
            [-0.125, 0.25],
            [0.015625 * (case % 3) as f32, 0.03125, -0.015625],
            if case & 4 != 0 { 0.0625 } else { 0. },
        );
        verify_mesh(&mut r, &m, case);
        assert_eq!(m.collision.len(), r.word() as usize);
        for cell in &m.collision {
            r.integers(cell.vertices.map(|v| v as i32));
            r.integers([
                cell.mode,
                cell.code as i32,
                cell.condition as i32,
                cell.value,
                cell.output as i32,
            ]);
            for event in cell.events {
                assert_eq!(event.map(|id| id as u32).unwrap_or(u32::MAX), r.word());
            }
        }
        assert_eq!(m.materials.len(), r.word() as usize);
        for material in &m.materials {
            let id = match material.source {
                MaterialSource::Null => 0,
                MaterialSource::Default => 1,
                MaterialSource::Operator(id) => id as u32 + 2,
            };
            assert_eq!(id, r.word());
            assert_eq!(material.pass, r.word() as i32);
        }
        assert_eq!(m.light_slots.len(), r.word() as usize);
        r.integers(m.light_slots.iter().map(|&v| v as i32));
        assert_eq!(m.parts.len(), r.word() as usize);
        r.integers(m.parts.iter().map(|&v| v as i32));
        assert_eq!(m.pivot.map(|v| v as i32).unwrap_or(-1), r.word() as i32);
        assert_eq!(m.got_normals, r.word() != 0);
    }
    assert!(r.0.is_empty());
}

#[test]
fn every_development_mesh_operator_has_a_rust_handler() {
    let document =
        kkrieger_oxide::kx::Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut count = 0;
    for op in &document.operators {
        let id = document.classes[op.class].id;
        if (0x81..=0xb3).contains(&id) {
            assert!(
                kkrieger_oxide::mesh_graph::implemented(id),
                "unported original mesh class 0x{id:02x}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 2531);
}

#[test]
#[ignore = "requires original named system fonts; execution check, not Windows GDI pixel parity"]
fn all_development_geometry_including_font_displacement_executes() {
    use kkrieger_oxide::{kx::Document, mesh_graph};
    let document = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut geometry = vec![false; document.operators.len()];
    for (i, op) in document.operators.iter().enumerate() {
        let id = document.classes[op.class].id;
        let inputs = if id == 0x8f {
            &op.inputs[..1]
        } else {
            &op.inputs
        };
        geometry[i] = mesh_graph::implemented(id)
            && op.animation == [1]
            && inputs.iter().all(|&input| geometry[input]);
    }
    assert_eq!(geometry.iter().filter(|&&g| g).count(), 2531);
    let mut roots = std::collections::BTreeSet::new();
    for (i, op) in document.operators.iter().enumerate() {
        if !geometry[i] {
            for &input in op.inputs.iter().chain(op.links.iter().flatten()) {
                if geometry[input] {
                    roots.insert(input);
                }
            }
        }
    }
    let roots: Vec<_> = roots.into_iter().collect();
    let meshes = mesh_graph::generate_many(&document, &roots).unwrap();
    assert!(!meshes.is_empty());
    for (&root, mesh) in roots.iter().zip(meshes) {
        assert!(
            mesh.vertices
                .iter()
                .flat_map(|v| v.attributes[0])
                .all(f32::is_finite),
            "op {root} positions"
        );
    }
}
