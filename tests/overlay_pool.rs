use kkrieger_oxide::scene::overlay::{COUNT, Pool, Viewport};
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
}
#[test]
fn original_ipp_target_allocation_references_and_viewports() {
    let mut r = Reader(include_bytes!("fixtures/overlay-pool.bin"));
    assert_eq!(r.word(), 32);
    assert_eq!(r.word(), 128);
    for _ in 0..32 {
        let mut pool = Pool::new(r.viewport(), std::array::from_fn(|i| 100 + i as i32));
        let mut resets = 0;
        for step in 0..128 {
            let code = r.word();
            let owner = r.word();
            let size = r.word() as usize;
            let index = r.word() as usize;
            let count = r.int();
            let mut vp = r.viewport();
            let owner = (owner != 0).then(|| owner as usize - 1);
            let result = match code {
                0 => {
                    pool.reset();
                    resets += 1;
                    None
                }
                1 => Some(pool.allocate(owner, size, count)),
                2 => Some(pool.get(size, index % 3).unwrap()),
                3 => pool.find(owner),
                4 => {
                    pool.free(index).unwrap();
                    None
                }
                5 => {
                    pool.set_master(vp);
                    None
                }
                6 => {
                    vp = pool.viewport(index).unwrap();
                    None
                }
                _ => unreachable!(),
            };
            assert_eq!(result, r.optional());
            assert_eq!(vp, r.viewport());
            for (i, target) in pool.targets.iter().enumerate() {
                let owner = r.word();
                assert_eq!(target.owner, (owner != 0).then(|| owner as usize - 1));
                assert_eq!(target.size, r.word() as usize);
                assert_eq!(target.references, r.int());
                if i < COUNT {
                    for scale in target.scale_uv {
                        assert_eq!(scale.to_bits(), r.word());
                    }
                }
            }
            assert_eq!(pool.master, r.viewport());
            assert_eq!(pool.last_output, r.optional());
            // Reset also requests two texture unbinds and one material bind from the
            // renderer. The pool implements allocation state independently of GPU I/O.
            assert_eq!(r.word(), resets * 2);
            assert_eq!(r.word(), resets);
            if step % 11 == 0 {
                pool.last_output = Some(index);
            }
        }
    }
    assert!(r.0.is_empty());
}
#[test]
fn full_pool_falls_back_to_screen_and_null_owners_are_not_cached() {
    use kkrieger_oxide::scene::overlay::{PER_SIZE, SCREEN};
    let mut pool = Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 1280, 720],
        },
        std::array::from_fn(|i| i as i32),
    );
    for i in 0..PER_SIZE {
        assert_eq!(pool.allocate(Some(i), 1, 1), PER_SIZE + i);
    }
    assert_eq!(pool.allocate(Some(9), 1, 2), SCREEN);
    pool.free(PER_SIZE + 1).unwrap();
    assert_eq!(pool.allocate(None, 1, 2), PER_SIZE + 1);
    assert_eq!(pool.find(None), None);
    assert!(pool.get(0, PER_SIZE).is_err());
    assert!(pool.free(SCREEN + 1).is_err());
    assert!(pool.viewport(SCREEN + 1).is_err());
}
