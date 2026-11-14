use kkrieger_oxide::{
    kx::{Document, Layout},
    material,
};

fn check(data: &[u8], layout: Layout) {
    let document = Document::parse_layout(data, layout).unwrap();
    let roots: Vec<_> = document
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(document.classes[op.class].id, 0xd0 | 0xd1))
        .map(|(i, _)| i)
        .collect();
    assert!(!roots.is_empty());
    let outputs = material::generate_many(&document, &roots).unwrap();
    assert_eq!(outputs.len(), roots.len());
    for output in outputs {
        for pass in &output.passes {
            assert_eq!(
                document.classes[document.operators[pass.shader.operator].class].id,
                0xd0
            );
            assert_eq!(pass.render_pass, pass.shader.parameters[49]);
            assert!(pass.size.is_finite() && pass.aspect.is_finite());
        }
    }
}

#[test]
fn development_material_graph_constructs() {
    check(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
    );
}

#[test]
#[ignore = "requires locally extracted original beta data"]
fn beta_material_graph_constructs() {
    check(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
    );
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        value
    }
    fn material(&mut self, material: &material::Material, id: usize) {
        assert_eq!(self.word() as usize, material.passes.len(), "operator {id}");
        assert_eq!(self.word() != 0, material.material11_insert);
        for pass in &material.passes {
            assert_eq!(
                material::Usage::from_raw(self.word()),
                pass.usage,
                "operator {id}"
            );
            let program = match pass.program {
                material::Program::Static => 1,
                material::Program::Shadow => 2,
                material::Program::Sprites => 3,
                material::Program::ThickLines => 4,
            };
            assert_eq!(self.word(), program);
            assert_eq!(self.word(), pass.render_pass);
            assert_eq!(self.word(), pass.size.to_bits());
            assert_eq!(self.word(), pass.aspect.to_bits());
            let state = pass.shader.state.borrow();
            for (i, &word) in state.words.iter().enumerate() {
                let expected = self.word();
                // The original CopyFrom leaves padding at constructor defaults.
                if ![3, 21, 22, 33].contains(&i) {
                    assert_eq!(word, expected, "operator {id}, word {i}");
                }
            }
            for texture in state.textures {
                assert_eq!(self.word(), texture.map(|i| i as u32).unwrap_or(u32::MAX));
            }
        }
    }
}
fn compare_graph(data: &[u8], layout: Layout, fixture: &[u8]) {
    let document = Document::parse_layout(data, layout).unwrap();
    let mut r = Reader(fixture);
    let count = r.word() as usize;
    let roots: Vec<_> = document
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(document.classes[op.class].id, 0xd0 | 0xd1))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(count, roots.len());
    let outputs = material::generate_many(&document, &roots).unwrap();
    for (&id, output) in roots.iter().zip(&outputs) {
        assert_eq!(r.word() as usize, id);
        r.material(output, id);
    }
    for (&id, output) in roots.iter().zip(&outputs) {
        let mut output = (**output).clone();
        if document.classes[document.operators[id].class].id == 0xd0 {
            output.execute(document.operators[id].parameters[..64].try_into().unwrap());
        }
        assert_eq!(r.word() as usize, id);
        r.material(&output, id);
    }
    assert!(r.0.is_empty());
}
#[test]
fn development_material_state_matches_original_cpp_graph() {
    compare_graph(
        include_bytes!("../assets/kkrieger3383.kx"),
        Layout::Legacy3383,
        include_bytes!("fixtures/material-graph3383.bin"),
    );
}
#[test]
#[ignore = "requires locally extracted original beta data and material oracle"]
fn beta_material_state_matches_original_cpp_graph() {
    compare_graph(
        &std::fs::read(".reference/verified-beta.kx").unwrap(),
        Layout::Beta2004,
        &std::fs::read(".reference/material-graph-beta.bin").unwrap(),
    );
}

#[test]
fn all_development_materials_generate_shader_sources() {
    let document = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let roots: Vec<_> = document
        .operators
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(document.classes[op.class].id, 0xd0 | 0xd1))
        .map(|(i, _)| i)
        .collect();
    for (&root, material) in roots
        .iter()
        .zip(material::generate_many(&document, &roots).unwrap())
    {
        for pass in &material.passes {
            pass.shader
                .state
                .borrow()
                .shader_sources()
                .unwrap_or_else(|e| panic!("operator {root}: {e}"));
        }
    }
}
