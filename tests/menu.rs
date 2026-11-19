use kkrieger_oxide::scene::menu;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
    fn check(&mut self, s: &[u8; 256], key: u32, vars: &[[f32; 4]; 4]) {
        for v in s {
            assert_eq!(u32::from(*v), self.word());
        }
        assert_eq!(key, self.word());
        for v in vars {
            assert_eq!(*v, self.vector());
        }
    }
}
#[test]
fn original_switch_menu_handlers_match_native_child_and_caller_states() {
    let mut r = Reader(include_bytes!("fixtures/menu.bin"));
    assert_eq!(r.word(), 256);
    for _ in 0..256 {
        let mut switches = std::array::from_fn(|_| r.word() as u8);
        let mut key = r.word();
        let mut vars = std::array::from_fn(|_| r.vector());
        let sw = r.word() as usize;
        let value = r.int();
        let flags = r.word();
        let output = r.word() as usize;
        let child = r.word() != 0;
        let result = menu::set_if(&mut switches, sw, value, flags, output).unwrap();
        assert_eq!(result.execute, child);
        let mut calls = 0;
        if result.execute {
            let saved = result.variable.map(|i| (i, vars[i]));
            if let Some(i) = result.variable {
                vars[i] = [u8::from(result.value) as f32; 4];
            }
            calls += 1;
            r.check(&switches, key, &vars);
            if let Some((i, v)) = saved {
                vars[i] = v;
            }
        }
        assert_eq!(calls, r.word());
        r.check(&switches, key, &vars);
        let sw = r.word() as usize;
        let value = r.int();
        let condition = r.int();
        let condition_switch = r.word() as usize;
        let condition_value = r.int();
        menu::state(
            &mut switches,
            &mut key,
            sw,
            value,
            condition,
            condition_switch,
            condition_value,
        )
        .unwrap();
        calls += 1;
        r.check(&switches, key, &vars);
        assert_eq!(calls, r.word());
        r.check(&switches, key, &vars);
        let sw = r.word() as usize;
        let max = r.int();
        let flags = r.word();
        menu::menu(&mut switches, &mut key, sw, max, flags).unwrap();
        calls += 1;
        r.check(&switches, key, &vars);
        assert_eq!(calls, r.word());
        r.check(&switches, key, &vars);
    }
    assert!(r.0.is_empty());
}
fn document() -> kkrieger_oxide::kx::Document<'static> {
    use kkrieger_oxide::kx::{Class, Document};
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0x0a, 0x14, 0x0b, 0x0e]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.splines.clear();
    }
    doc.operators[0].inputs = vec![1];
    doc
}
#[test]
fn scene_setif_scopes_the_child_variable_and_restores_it_on_error() {
    use kkrieger_oxide::scene::exec::Execution;
    let mut doc = document();
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![5, 3, 0, 0];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![0, 8f32.to_bits(), 1f32.to_bits()];
    doc.operators[1].animation = &[2, 0, 0xa1, 0, 1];
    let mut execution = Execution::new(&doc);
    execution.switches[5] = 3;
    execution.vm.variables[0] = [9., 8., 7., 6.];
    execution.frame(&doc, 0).unwrap();
    assert!(execution.splash.enabled);
    assert_eq!(execution.splash.hits, 1);
    assert_eq!(execution.vm.variables[0], [9., 8., 7., 6.]);
    doc.classes[1].id = 0xffff;
    assert!(execution.frame(&doc, 0).is_err());
    assert_eq!(execution.vm.variables[0], [9., 8., 7., 6.]);
}
#[test]
fn scene_state_consumes_key_before_child_menu() {
    use kkrieger_oxide::{game::input::RIGHT, scene::exec::Execution};
    let mut doc = document();
    doc.operators[0].class = 2;
    doc.operators[0].parameters = vec![7, 2, 7, 6, 1];
    doc.operators[1].class = 3;
    doc.operators[1].parameters = vec![9, 5, 0];
    let mut execution = Execution::new(&doc);
    execution.game.last_key = RIGHT;
    execution.switches[6] = 1;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.switches[7], 2);
    assert_eq!(execution.switches[9], 0);
    assert_eq!(execution.game.last_key, 0);
}

#[test]
fn released_state_keeps_mouse_key_and_still_executes_its_child() {
    use kkrieger_oxide::{game::input::MOUSE_LEFT, kx::Layout, scene::exec::Execution};
    let mut doc = document();
    doc.layout = Layout::Beta2004;
    doc.operators[0].class = 2;
    doc.operators[0].parameters = vec![7, 2, 2, 6, 1];
    doc.operators[1].class = 2;
    doc.operators[1].parameters = vec![9, 5, 0, 6, 1];
    let mut execution = Execution::new(&doc);
    execution.game.last_key = MOUSE_LEFT;
    execution.switches[6] = 1;
    execution.frame(&doc, 0).unwrap();
    assert_eq!(execution.switches[7], 0);
    assert_eq!(execution.switches[9], 5);
    assert_eq!(execution.game.last_key, MOUSE_LEFT);
}
