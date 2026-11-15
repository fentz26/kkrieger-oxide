use kkrieger_oxide::{
    collision::{
        Cell, DynamicCell, connectivity,
        ray::Info,
        world::{Entry, Logic, World},
    },
    mesh::identity,
};
use std::{cell::RefCell, rc::Rc};
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
    fn cell(&mut self) -> Cell {
        let mode = self.word();
        Cell::from_vertices(std::array::from_fn(|_| self.vector()), mode)
    }
    fn info(&mut self, dynamic: &[Rc<RefCell<DynamicCell>>], base: usize) -> Info {
        let plane = self.vector();
        let position = self.vector();
        let distance = f32::from_bits(self.word());
        let radius = f32::from_bits(self.word());
        let zone_mask = self.word();
        let flags = self.word();
        let cell = self.word();
        Info {
            plane,
            position,
            distance,
            radius,
            zone_mask,
            flags,
            dynamic_cell: if cell == u32::MAX {
                None
            } else {
                Some(Rc::clone(&dynamic[cell as usize - base]))
            },
        }
    }
}

#[test]
fn original_connected_ray_traversal_and_zone_side_effects_match() {
    let mut r = Reader(include_bytes!("fixtures/collision-ray.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 20);
    for case in 0..64 {
        let count = r.word() as usize;
        let dynamic_count = r.word();
        let cells: Vec<_> = (0..count).map(|_| r.cell()).collect();
        let dynamic: Vec<_> = (0..dynamic_count)
            .map(|_| Rc::new(RefCell::new(DynamicCell::new(r.cell(), identity()))))
            .collect();
        let graph = connectivity::connect(&cells);
        let mut world = World {
            entries: cells
                .into_iter()
                .map(|cell| Entry {
                    cell,
                    sector: None,
                    events: [None, None],
                    logic: Logic {
                        code: 0,
                        condition: 0,
                        value: 0,
                        output: 0,
                        events: [None, None],
                    },
                })
                .collect(),
            adds: (0..18).collect(),
            subs: vec![18, 20],
            zones: vec![19, 21],
            connectivity: graph,
            faces: (0..count)
                .map(|i| if i < 18 { Some(vec![]) } else { None })
                .collect(),
        };
        for query in 0..20 {
            let mut add = r.word() as usize;
            let end = r.vector();
            let start = r.vector();
            let mut info = r.info(&dynamic, count);
            let hit = world
                .collide_ray(&mut add, end, start, &mut info, &dynamic)
                .unwrap();
            assert_eq!(hit, r.word() != 0, "{case}/{query}");
            assert_eq!(add, r.word() as usize, "{case}/{query}");
            let expected = r.info(&dynamic, count);
            for (a, b) in info
                .plane
                .into_iter()
                .chain(info.position)
                .chain([info.distance, info.radius])
                .zip(
                    expected
                        .plane
                        .into_iter()
                        .chain(expected.position)
                        .chain([expected.distance, expected.radius]),
                )
            {
                assert_eq!(a.to_bits(), b.to_bits(), "{case}/{query}: {a} vs {b}");
            }
            assert_eq!(info.zone_mask, expected.zone_mask);
            assert_eq!(info.flags, expected.flags);
            assert!(
                match (&info.dynamic_cell, &expected.dynamic_cell) {
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    (None, None) => true,
                    _ => false,
                },
                "{case}/{query}: dynamic cell"
            );
            for &i in &world.connectivity.sorted {
                let state = world.entries[i].cell.interactions;
                assert_eq!(
                    [state.enter, state.leave, state.collided],
                    [r.word(), r.word(), r.word()],
                    "{case}/{query} static {i}"
                );
            }
            for (i, cell) in dynamic.iter().enumerate() {
                let state = cell.borrow().cell.interactions;
                assert_eq!(
                    [state.enter, state.leave, state.collided],
                    [r.word(), r.word(), r.word()],
                    "{case}/{query} dynamic {i}"
                );
            }
        }
    }
    assert!(r.0.is_empty());
}
