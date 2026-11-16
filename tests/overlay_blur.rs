use kkrieger_oxide::scene::overlay::{Blur, Draw, Pool, Viewport};
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
    fn draws(&mut self, draws: &[Draw], inputs: u32) {
        assert_eq!(self.word() as usize, draws.len() / 2);
        assert_eq!(self.word() as usize, draws.len());
        assert_eq!(self.word() as usize, draws.len());
        assert_eq!(self.word(), inputs);
        assert_eq!(self.word() as usize, draws.len());
        for draw in draws {
            self.draw(draw);
        }
    }
}
#[test]
fn original_four_sample_blends_and_staged_blur_packets_match_native() {
    let mut r = Reader(include_bytes!("fixtures/overlay-blur.bin"));
    assert_eq!(r.word(), 512);
    for _ in 0..512 {
        let pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        let source = r.word() as usize;
        let destination = r.word() as usize;
        let coordinate_mode = r.word() != 0;
        let amplify = r.float();
        let mut rects: [[f32; 4]; 4] = std::array::from_fn(|_| std::array::from_fn(|_| r.float()));
        let draws = pool
            .blend4x(source, destination, &mut rects, coordinate_mode, amplify)
            .unwrap();
        for rect in rects {
            for value in rect {
                assert_eq!(r.word(), value.to_bits());
            }
        }
        r.draws(&draws, 0);
    }
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
            let flags = r.word();
            let stages = r.int();
            let count = r.int();
            let radius = r.float();
            let amplify = r.float();
            pool.last_output = (source < 13).then_some(source);
            let draws = pool
                .blur_operator(
                    owner,
                    &Blur {
                        size,
                        radius,
                        amplify,
                        flags,
                        stages,
                        count,
                    },
                )
                .unwrap();
            r.draws(&draws, 1);
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
fn staged_blur_executes_inputs_before_cache_lookup_and_uses_screen_intermediates() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    for screen in [false, true] {
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        doc.events.clear();
        doc.classes = [0xe3, 0xea, 0xe6]
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
        doc.operators[0].parameters = vec![
            if screen { 4 } else { 0 },
            8f32.to_bits(),
            2f32.to_bits(),
            0,
            3,
            1,
        ];
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
        assert_eq!(e.overlay_draws.len(), 6);
        for (index, draw) in e.overlay_draws.iter().enumerate() {
            assert_eq!(draw.material, 4 + index % 2);
            assert_eq!(draw.color, if index < 4 { 0x40000000 } else { 0x80000000 });
            assert_eq!(
                draw.viewport.render_target,
                if index < 2 {
                    100
                } else if index < 4 {
                    101
                } else if screen {
                    -1
                } else {
                    100
                }
            );
        }
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.targets[source].references, 3);
        assert_eq!(pool.last_output, Some(if screen { 12 } else { 0 }));
        e.frame(&doc, 0).unwrap();
        assert_eq!(e.overlay_draws.len(), if screen { 6 } else { 0 });
        assert_eq!(
            e.overlay.as_ref().unwrap().targets[source].references,
            if screen { 4 } else { 5 }
        );
    }
}
