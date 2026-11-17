use kkrieger_oxide::scene::overlay::{Draw, Pool, Viewport};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        n
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn int(&mut self) -> i32 {
        self.word() as i32
    }
    fn optional(&mut self) -> Option<usize> {
        let n = self.word();
        (n != u32::MAX).then_some(n as usize)
    }
    fn viewport(&mut self) -> Viewport {
        Viewport {
            screen: self.int(),
            render_target: self.int(),
            window: std::array::from_fn(|_| self.int()),
        }
    }
    fn draw(&mut self, draw: &Draw) {
        assert_eq!(self.viewport(), draw.viewport);
        assert_eq!(self.int(), draw.environment.orthogonal);
        assert_eq!(self.word() as usize, draw.material);
        for texture in [Some(draw.texture), draw.texture1, draw.texture2] {
            assert_eq!(self.int(), texture.unwrap_or(-1));
        }
        assert_eq!(self.word(), draw.color);
        assert_eq!(self.word(), draw.color2.unwrap());
        for vertex in draw.vertices {
            for value in vertex {
                assert_eq!(self.word(), value.to_bits());
            }
        }
    }
}
#[test]
fn original_staged_sharpen_draws_cache_and_ownership_match_native() {
    let mut r = Reader(include_bytes!("fixtures/overlay-sharpen.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 64);
    for _ in 0..32 {
        let mut pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        for _ in 0..64 {
            if r.word() != 0 {
                pool.reset();
            }
            let source = r.word() as usize;
            let owner = r.word() as usize;
            let size = r.word() as usize;
            let amplify = r.word();
            let stages = r.int();
            let count = r.int();
            let radius = r.float();
            pool.last_output = (source < 13).then_some(source);
            let draws = pool
                .sharpen_operator(owner, size, radius, amplify, stages, count)
                .unwrap();
            let calls = r.word() as usize;
            assert_eq!(calls, draws.len().div_ceil(2));
            assert_eq!(r.word() as usize, draws.len());
            assert_eq!(r.word() as usize, draws.len());
            assert_eq!(r.word(), 1);
            assert_eq!(r.word() as usize, draws.len());
            for draw in draws {
                r.draw(&draw);
            }
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
fn sharpen_executes_inputs_before_cached_output_lookup() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = [0xe5, 0xea, 0xe6]
        .into_iter()
        .map(|id| Class {
            id,
            convention: 0,
            packing: &[],
        })
        .collect();
    doc.operators.truncate(3);
    for (index, op) in doc.operators.iter_mut().enumerate() {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.class = index;
    }
    doc.operators[0].parameters = vec![0, 8f32.to_bits(), 0x00808080, 3, 1];
    doc.operators[0].inputs = vec![1];
    doc.operators[1].parameters = vec![3];
    doc.operators[1].inputs = vec![2];
    doc.operators[2].parameters = vec![0; 5];
    let mut e = Execution::new(&doc);
    let mut pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    );
    let source = pool.allocate(Some(2), 1, 2);
    e.overlay = Some(pool);
    e.frame(&doc, 0).unwrap();
    assert_eq!(e.overlay_draws.len(), 7);
    assert_eq!(e.overlay_draws.last().unwrap().material, 7);
    assert_eq!(e.overlay_draws.last().unwrap().color2, Some(0x00808080));
    let pool = e.overlay.as_ref().unwrap();
    assert_eq!(pool.targets[source].references, 3);
    let output = pool.last_output;
    e.frame(&doc, 0).unwrap();
    assert!(e.overlay_draws.is_empty());
    let pool = e.overlay.as_ref().unwrap();
    assert_eq!(pool.last_output, output);
    assert_eq!(pool.targets[source].references, 5);
}
