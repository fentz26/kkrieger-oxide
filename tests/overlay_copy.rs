use kkrieger_oxide::scene::overlay::{CopyDraw, Pool, Viewport, fx_quad};
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
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
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
    fn draw(&mut self, draw: Option<&CopyDraw>, inputs: u32) {
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
fn original_effect_quad_geometry_and_copy_operator_cache() {
    let mut r = Reader(include_bytes!("fixtures/overlay-copy.bin"));
    assert_eq!(r.word(), 512);
    for _ in 0..512 {
        let mask = r.word();
        let material = r.word();
        let scales: [f32; 4] = std::array::from_fn(|_| r.float());
        let uv: [[f32; 4]; 2] = std::array::from_fn(|_| std::array::from_fn(|_| r.float()));
        let quad = fx_quad(
            (mask & 1 != 0).then_some([scales[0], scales[1]]),
            (mask & 2 != 0).then_some(uv[0]),
            (mask & 4 != 0).then_some([scales[2], scales[3]]),
            (mask & 8 != 0).then_some(uv[1]),
        );
        for n in [0, 1, 1, 1, material, u32::MAX, 0] {
            assert_eq!(r.word(), n);
        }
        r.vertices(quad);
        assert_eq!(r.word(), 0);
    }
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 64);
    for _ in 0..32 {
        let mut pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        for _ in 0..64 {
            if r.word() != 0 {
                pool.reset();
            }
            let kind = r.word();
            let source = r.word() as usize;
            let destination = r.word() as usize;
            let owner = r.word() as usize;
            let size = r.word() as usize;
            let mode = r.word() as usize;
            let color = r.word();
            let count = r.int();
            let zoom = r.float();
            let draw = if kind == 0 {
                pool.copy(source % 13, destination, mode, color, zoom)
                    .unwrap()
            } else {
                pool.last_output = (source < 13).then_some(source);
                pool.copy_operator(owner, size, color, zoom, count).unwrap()
            };
            r.draw(draw.as_ref(), kind);
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
fn scene_copy_executes_inputs_then_emits_the_original_cached_copy() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xe2,
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
    doc.operators[0].parameters = vec![0, 0xff123456, 1f32.to_bits(), 1];
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
    assert_eq!(e.overlay_copies.len(), 1);
    let draw = &e.overlay_copies[0];
    assert_eq!(draw.texture, 103);
    assert_eq!(draw.color, 0xff123456);
    assert_eq!(draw.material, 27);
    let pool = e.overlay.as_ref().unwrap();
    assert_eq!(pool.last_output, Some(0));
    assert_eq!(pool.targets[source].references, 2);
    e.frame(&doc, 0).unwrap();
    assert!(e.overlay_copies.is_empty());
    assert_eq!(e.overlay.as_ref().unwrap().last_output, Some(0));
}
