use kkrieger_oxide::scene::overlay::{Draw, Pool, Viewport};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn viewport(&mut self) -> Viewport {
        Viewport {
            screen: self.int(),
            render_target: self.int(),
            window: std::array::from_fn(|_| self.int()),
        }
    }
    fn optional(&mut self) -> Option<usize> {
        let n = self.word();
        (n != u32::MAX).then_some(n as usize)
    }
    fn vertices(&mut self, vertices: [[f32; 8]; 4]) {
        for vertex in vertices {
            for value in vertex {
                assert_eq!(value.to_bits(), self.word());
            }
        }
    }
    fn draw(&mut self, draw: Option<&Draw>, inputs: u32) {
        let exists = u32::from(draw.is_some());
        for _ in 0..3 {
            assert_eq!(self.word(), exists);
        }
        if let Some(draw) = draw {
            assert_eq!(self.int(), draw.environment.orthogonal);
            assert_eq!(self.word() as usize, draw.material);
            assert_eq!(self.int(), draw.texture);
            assert_eq!(self.word(), draw.color);
            assert_eq!(self.viewport(), draw.viewport);
            self.vertices(draw.vertices);
        } else {
            for _ in 0..3 {
                assert_eq!(self.int(), -1);
            }
            assert_eq!(self.word(), 0);
        }
        assert_eq!(self.word(), inputs);
    }
}
#[test]
fn original_color_operator_cache_input_order_and_reference_consumption() {
    let mut r = Reader(include_bytes!("fixtures/overlay-color.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 128);
    for _ in 0..32 {
        let mut pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        for _ in 0..128 {
            if r.word() != 0 {
                pool.reset();
            }
            let source = r.word() as usize;
            let owner = r.word() as usize;
            let size = r.word() as usize;
            let operation = r.word() as usize;
            let color = r.word();
            let amplify = r.word();
            let count = r.int();
            let cached = pool.find(Some(owner));
            if cached.is_none() {
                pool.last_output = (source < 13).then_some(source);
            }
            let draw = pool
                .color_operator(owner, size, color, operation, amplify, count)
                .unwrap();
            r.draw(draw.as_ref(), u32::from(cached.is_none()));
            assert_eq!(
                r.word(),
                draw.as_ref().and_then(|draw| draw.color2).unwrap_or(0)
            );
            assert_eq!(pool.last_output, r.optional());
            for target in &pool.targets {
                let owner = r.word();
                assert_eq!(target.owner, (owner != 0).then(|| owner as usize - 1));
                assert_eq!(target.references, r.int());
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn scene_color_skips_inputs_for_a_cached_output() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xe6,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0x12,
            convention: 0x1000_0000,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
    }
    doc.operators[0].class = 0;
    doc.operators[0].parameters = vec![0, 0xff123456, 4, 0xffabcdef, 1];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    doc.operators[1].parameters = vec![1];
    doc.operators[1].links = vec![Some(0); 8];
    let mut e = Execution::new(&doc);
    let mut pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    );
    let source = pool.allocate(None, 1, 2);
    pool.last_output = Some(source);
    e.overlay = Some(pool);
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.game.events.optics, [Some(0); 8]);
    assert_eq!(e.overlay_draws.len(), 1);
    let draw = &e.overlay_draws[0];
    assert_eq!(draw.texture, 103);
    assert_eq!(draw.color, 0xff123456);
    assert_eq!(draw.material, 12);
    assert_eq!(draw.color2, Some(0xffabcdef));
    let pool = e.overlay.as_ref().unwrap();
    assert_eq!(pool.last_output, Some(0));
    assert_eq!(pool.targets[source].references, 1);
    e.game.events.optics = [None; 8];
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.game.events.optics, [None; 8]);
    assert!(e.overlay_draws.is_empty());
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(0));
}
