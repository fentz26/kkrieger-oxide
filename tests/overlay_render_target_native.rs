use kkrieger_oxide::{
    kx::{Class, Document},
    scene::{
        exec::Execution,
        overlay::{Draw, Pool, Viewport},
    },
};
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
fn original_render_target_wrapper_matches_native_draws_and_restored_pool() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0xeb, 0xea, 0x26]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0x1000_0000,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(3);
    for (index, op) in doc.operators.iter_mut().enumerate() {
        op.animation = &[1];
        op.class = index;
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![];
    }
    doc.operators[0].links = vec![Some(2)];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].parameters = vec![1];
    let mut r = Reader(include_bytes!("fixtures/overlay-render-target.bin"));
    assert_eq!(r.word(), 256);
    for _ in 0..256 {
        let master = r.viewport();
        let texture = r.int();
        let width = r.int();
        let height = r.int();
        let source = r.word() as usize;
        let mut e = Execution::new(&doc);
        let mut pool = Pool::new(master, std::array::from_fn(|i| 100 + i as i32));
        pool.last_output = if texture == -1 {
            Some(0)
        } else {
            (source < 13).then_some(source)
        };
        e.overlay = Some(pool);
        e.render_textures.insert(
            2,
            Viewport {
                screen: -1,
                render_target: texture,
                window: [0, 0, width, height],
            },
        );
        e.frame(&doc, 0).unwrap();
        assert!(e.overlay_draws.len() <= 1);
        r.draw(e.overlay_draws.first(), u32::from(texture != -1));
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.master, r.viewport());
        assert_eq!(pool.last_output, r.optional());
        for target in &pool.targets {
            assert_eq!(target.references, r.int());
            if target.size < 4 {
                for value in target.scale_uv {
                    assert_eq!(value.to_bits(), r.word());
                }
            }
        }
    }
    assert!(r.0.is_empty());
}
