use kkrieger_oxide::scene::overlay::{Draw, Pool, Viewport, fx_quad};
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
fn original_mask_two_passes_early_exits_cache_and_three_reference_consumptions() {
    let mut r = Reader(include_bytes!("fixtures/overlay-mask.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 128);
    for _ in 0..32 {
        let mut pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        for _ in 0..128 {
            if r.word() != 0 {
                pool.reset();
            }
            let inputs = [r.word() as usize, r.word() as usize, r.word() as usize];
            let owner = r.word() as usize;
            let size = r.word() as usize;
            let count = r.int();
            let mut input_calls = 0;
            let mut trace = 0;
            let mut sources = Some([0; 3]);
            if pool.find(Some(owner)).is_none() {
                for (index, input) in inputs.into_iter().enumerate() {
                    pool.last_output = None;
                    input_calls += 1;
                    trace = (trace << 4) | (index + 1);
                    pool.last_output = (input < 13).then_some(input);
                    if let Some(value) = pool
                        .last_output
                        .filter(|&output| pool.targets[output].texture.is_some())
                    {
                        sources.as_mut().unwrap()[index] = value;
                    } else {
                        sources = None;
                        break;
                    }
                }
            }
            let draws = if let Some(sources) = sources {
                pool.mask_operator(owner, sources, size, count).unwrap()
            } else {
                vec![]
            };
            assert_eq!(r.word(), u32::from(!draws.is_empty()));
            assert_eq!(r.word() as usize, draws.len());
            assert_eq!(r.word() as usize, draws.len());
            assert_eq!(r.word(), input_calls);
            assert_eq!(r.word() as usize, trace);
            assert_eq!(r.word() as usize, draws.len());
            for draw in &draws {
                r.draw(draw);
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
fn scene_mask_runs_both_passes_in_order_and_releases_aliased_inputs() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::exec::Execution,
    };
    for alias in [false, true] {
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        doc.events.clear();
        doc.classes = vec![
            Class {
                id: 0xe8,
                convention: 0,
                packing: &[],
            },
            Class {
                id: 0xe6,
                convention: 0,
                packing: &[],
            },
        ];
        doc.operators.truncate(4);
        for op in &mut doc.operators {
            op.animation = &[1];
            op.inputs.clear();
            op.links.clear();
            op.class = 1;
            op.parameters = vec![0; 5];
        }
        doc.operators[0].class = 0;
        doc.operators[0].parameters = vec![2, 1];
        doc.operators[0].inputs = if alias { vec![1, 1, 1] } else { vec![1, 2, 3] };
        let mut e = Execution::new(&doc);
        let mut pool = Pool::new(
            Viewport {
                screen: 0,
                render_target: -1,
                window: [0, 0, 640, 360],
            },
            std::array::from_fn(|i| 100 + i as i32),
        );
        let a = pool.allocate(Some(1), 1, 3);
        let b = if alias {
            a
        } else {
            pool.allocate(Some(2), 3, 3)
        };
        let mask = if alias {
            a
        } else {
            pool.allocate(Some(3), 0, 3)
        };
        let scales = [
            pool.targets[a].scale_uv,
            pool.targets[b].scale_uv,
            pool.targets[mask].scale_uv,
        ];
        e.overlay = Some(pool);
        e.frame(&doc, 0).unwrap();
        assert_eq!(e.overlay_draws.len(), 2);
        for (index, draw) in e.overlay_draws.iter().enumerate() {
            assert_eq!(draw.material, 25 + index);
            assert_eq!(draw.texture, 100 + mask as i32);
            assert_eq!(draw.color, 0x408040);
            if index == 0 {
                assert_eq!(draw.texture1, Some(100 + b as i32));
                assert_eq!(draw.texture2, None);
                assert_eq!(
                    draw.vertices,
                    fx_quad(Some(scales[2]), None, Some(scales[1]), None)
                );
            } else {
                assert_eq!(draw.texture1, None);
                assert_eq!(draw.texture2, Some(100 + a as i32));
                assert_eq!(
                    draw.vertices,
                    fx_quad(Some(scales[2]), None, Some(scales[0]), None)
                );
            }
        }
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.last_output, Some(6));
        for source in [a, b, mask] {
            assert_eq!(pool.targets[source].references, if alias { 0 } else { 2 });
        }
        doc.classes[1].id = 0xffff;
        e.frame(&doc, 0).unwrap();
        assert!(e.overlay_draws.is_empty());
    }
}
#[test]
fn failed_mask_preserves_screen_output_while_merge_clears_it() {
    use kkrieger_oxide::{
        kx::{Class, Document},
        scene::{exec::Execution, overlay::SCREEN},
    };
    for mask in [false, true] {
        let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
        doc.events.clear();
        doc.classes = [if mask { 0xe8 } else { 0xe7 }, 0xe2, 0xe6, 0xea, 0xffff]
            .into_iter()
            .map(|id| Class {
                id,
                convention: 0,
                packing: &[],
            })
            .collect();
        doc.operators.truncate(5);
        for (index, op) in doc.operators.iter_mut().enumerate() {
            op.animation = &[1];
            op.inputs.clear();
            op.links.clear();
            op.class = index;
            op.parameters = vec![0; 5];
        }
        doc.operators[0].parameters = if mask {
            vec![0, 1]
        } else {
            vec![0, 0, 127, 0xffffffff, 1]
        };
        doc.operators[0].inputs = vec![1, 4, 4];
        doc.operators[1].parameters = vec![4, 0xffffffff, 1f32.to_bits(), 1];
        doc.operators[1].inputs = vec![2];
        doc.operators[3].parameters = vec![3];
        doc.operators[3].inputs = vec![0];
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
        e.frame(&doc, 3).unwrap();
        let pool = e.overlay.as_ref().unwrap();
        assert_eq!(pool.last_output, if mask { Some(SCREEN) } else { None });
        assert_eq!(pool.targets[SCREEN].references, if mask { 2 } else { 0 });
        assert_eq!(pool.targets[source].references, 2);
        assert_eq!(e.overlay_draws.len(), 1);
        assert_eq!(e.overlay_draws[0].material, 27);
    }
}
