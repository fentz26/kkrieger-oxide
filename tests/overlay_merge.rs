use kkrieger_oxide::scene::overlay::{Draw, Merge, Pool, Viewport};
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
fn original_merge_input_order_early_exit_dual_uvs_and_reference_consumption() {
    let mut r = Reader(include_bytes!("fixtures/overlay-merge.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 128);
    for _ in 0..32 {
        let mut pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        for _ in 0..128 {
            if r.word() != 0 {
                pool.reset();
            }
            let inputs = [r.word() as usize, r.word() as usize];
            let owner = r.word() as usize;
            let parameters = Merge {
                size: r.word() as usize,
                operation: r.word() as usize,
                alpha: r.int(),
                amplify: r.word(),
                count: r.int(),
            };
            let mut input_calls = 0;
            let mut trace = 0;
            let mut sources = Some([0; 2]);
            if pool.find(Some(owner)).is_none() {
                for (i, input) in inputs.into_iter().enumerate() {
                    pool.last_output = None;
                    input_calls += 1;
                    trace = (trace << 4) | (i + 1);
                    pool.last_output = (input < 13).then_some(input);
                    let value = pool.last_output.take();
                    if let Some(value) =
                        value.filter(|&index| pool.targets[index].texture.is_some())
                    {
                        sources.as_mut().unwrap()[i] = value;
                    } else {
                        sources = None;
                        break;
                    }
                }
            }
            let draw = if let Some(sources) = sources {
                pool.merge_operator(owner, sources, &parameters).unwrap()
            } else {
                None
            };
            r.draw(draw.as_ref(), input_calls);
            assert_eq!(
                r.word(),
                draw.as_ref().and_then(|draw| draw.color2).unwrap_or(0)
            );
            assert_eq!(
                r.int(),
                draw.as_ref().and_then(|draw| draw.texture1).unwrap_or(-1)
            );
            assert_eq!(r.word() as usize, trace);
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
fn scene_merge_executes_two_cached_color_inputs_and_preserves_alias_counts() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::{exec::Execution, overlay::fx_quad},
    };
    for alias in [false, true] {
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        doc.events.clear();
        doc.classes = vec![
            Class {
                id: 0xe7,
                convention: 0,
                packing: &[],
            },
            Class {
                id: 0xe6,
                convention: 0,
                packing: &[],
            },
        ];
        doc.operators.truncate(3);
        for op in &mut doc.operators {
            op.animation = &[1];
            op.inputs.clear();
            op.links.clear();
            op.class = 1;
            op.parameters = vec![0; 5];
        }
        doc.operators[0].class = 0;
        doc.operators[0].inputs = vec![1, if alias { 1 } else { 2 }];
        doc.operators[0].parameters = vec![0, 8, 127, 0xffabcdef, 1];
        let mut e = Execution::new(&doc);
        let mut pool = Pool::new(
            Viewport {
                screen: 0,
                render_target: -1,
                window: [0, 0, 640, 360],
            },
            std::array::from_fn(|i| 100 + i as i32),
        );
        let first = pool.allocate(Some(1), 1, 2);
        let second = if alias {
            first
        } else {
            pool.allocate(Some(2), 3, 2)
        };
        let scales = [pool.targets[first].scale_uv, pool.targets[second].scale_uv];
        e.overlay = Some(pool);
        e.frame(&doc, 0).unwrap();
        assert_eq!(e.overlay_draws.len(), 1);
        let draw = &e.overlay_draws[0];
        assert_eq!(draw.material, 24);
        assert_eq!(draw.texture, 100 + first as i32);
        assert_eq!(draw.texture1, Some(100 + second as i32));
        assert_eq!(draw.color, 0x7f000000);
        assert_eq!(draw.color2, Some(0xffabcdef));
        assert_eq!(
            draw.vertices,
            fx_quad(Some(scales[0]), None, Some(scales[1]), None)
        );
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.last_output, Some(0));
        assert_eq!(pool.targets[first].references, if alias { 0 } else { 1 });
        if !alias {
            assert_eq!(pool.targets[second].references, 1);
        }
        // A cached merge skips both children even if they could no longer run.
        doc.classes[1].id = 0xffff;
        e.frame(&doc, 0).unwrap();
        assert!(e.overlay_draws.is_empty());
    }
}

#[test]
fn missing_merge_input_clears_output_without_consuming_other_references() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.events.clear();
    doc.classes = vec![
        Class {
            id: 0xe7,
            convention: 0,
            packing: &[],
        },
        Class {
            id: 0xe6,
            convention: 0,
            packing: &[],
        },
    ];
    doc.operators.truncate(2);
    for op in &mut doc.operators {
        op.animation = &[1];
        op.inputs.clear();
        op.links.clear();
        op.parameters = vec![0; 5];
    }
    doc.operators[0].class = 0;
    doc.operators[0].inputs = vec![1];
    doc.operators[1].class = 1;
    let mut e = Execution::new(&doc);
    let mut pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    );
    let source = pool.allocate(Some(1), 1, 2);
    pool.last_output = Some(source);
    e.overlay = Some(pool);
    e.frame(&doc, 0).unwrap();
    let pool = e.overlay.as_ref().unwrap();
    assert_eq!(pool.last_output, None);
    assert_eq!(pool.targets[source].references, 2);
    assert!(e.overlay_draws.is_empty());
}
