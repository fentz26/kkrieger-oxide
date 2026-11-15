use kkrieger_oxide::{
    collision::{
        Cell, DynamicCell, connectivity, faces,
        movement::Collider,
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
}
#[test]
fn original_complete_collider_movement_matches_faces_exclusions_and_zones() {
    let mut r = Reader(include_bytes!("fixtures/collision-movement.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 20);
    for case in 0..64 {
        let count = r.word();
        let dynamic_count = r.word();
        let cells: Vec<_> = (0..count).map(|_| r.cell()).collect();
        let dynamic: Vec<_> = (0..dynamic_count)
            .map(|_| {
                let cell = r.cell();
                let mode = cell.mode;
                let mut cell = DynamicCell::new(cell, identity());
                cell.cell.mode = mode;
                cell.monster = (r.word() != 0).then_some(17);
                Rc::new(RefCell::new(cell))
            })
            .collect();
        let graph = connectivity::connect(&cells);
        let cell_faces: Vec<_> = cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if c.mode & 3 == 0 {
                    Some(faces::create(&cells, i, &graph.neighbors[i].adds).unwrap())
                } else {
                    None
                }
            })
            .collect();
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
            adds: vec![0, 1],
            subs: vec![2],
            zones: vec![3],
            connectivity: graph,
            faces: cell_faces,
        };
        for query in 0..20 {
            let mut collider = Collider {
                position: r.vector(),
                radius: f32::from_bits(r.word()),
                cell: r.word() as usize,
            };
            let movement = r.vector();
            let who = r.word();
            let result = world
                .move_collider(&mut collider, movement, who, &dynamic, Some(&dynamic[0]))
                .unwrap();
            assert_eq!(result, r.word() != 0, "{case}/{query}");
            for (a, b) in collider.position.into_iter().zip(r.vector()) {
                assert_eq!(a.to_bits(), b.to_bits(), "{case}/{query}: {a} vs {b}");
            }
            assert_eq!(collider.radius.to_bits(), r.word());
            assert_eq!(collider.cell, r.word() as usize, "{case}/{query}");
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
