use kkrieger_oxide::{
    collision::world::World,
    mesh::{Collision, Mesh, Vertex},
    scene::{Handle, Scene},
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        value
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| f32::from_bits(self.word()))
    }
    fn optional(&mut self) -> Option<usize> {
        let value = self.word();
        (value != u32::MAX).then_some(value as usize)
    }
}

#[test]
fn original_scene_traversal_preserves_instances_sectors_logic_and_events() {
    let mut r = Reader(include_bytes!("fixtures/collision-world.bin"));
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let mut mesh = Mesh::default();
        let count = r.word();
        for _ in 0..count {
            let mode = r.word() as i32;
            let first = mesh.vertices.len();
            for _ in 0..8 {
                let mut attributes = [[0.; 4]; 5];
                attributes[0] = r.vector();
                mesh.vertices.push(Vertex {
                    element: Default::default(),
                    next: 0,
                    first: 0,
                    temp: 0,
                    temp2: 0,
                    reindex: 0,
                    weight: [0; 4],
                    matrix: [0; 4],
                    attributes,
                });
            }
            let code = r.word() as u8;
            let condition = r.word() as u8;
            let value = r.word() as i32;
            let output = r.word() as u8;
            mesh.collision.push(Collision {
                vertices: std::array::from_fn(|i| first + i),
                mode,
                code,
                condition,
                value,
                output,
                events: [r.optional(), r.optional()],
            });
        }
        let count = r.word();
        let mut children = Vec::new();
        let nodes: Vec<Handle> = (0..count)
            .map(|_| {
                let srt = std::array::from_fn(|_| f32::from_bits(r.word()));
                let count = r.word() as i32;
                let is_sector = r.word() != 0;
                let n = r.word();
                children.push((0..n).map(|_| r.word() as usize).collect::<Vec<_>>());
                Rc::new(RefCell::new(Scene {
                    srt,
                    count,
                    is_sector,
                    mesh: Some(0),
                    ..Default::default()
                }))
            })
            .collect();
        for (node, children) in nodes.iter().zip(children) {
            node.borrow_mut().children =
                children.into_iter().map(|i| Rc::clone(&nodes[i])).collect();
        }
        let sectors: Vec<_> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.borrow().is_sector)
            .map(|(i, node)| (i, Rc::clone(node)))
            .collect();
        let world = World::build(
            0,
            &nodes[0],
            &BTreeMap::from([(0, Rc::new(mesh))]),
            &sectors,
        )
        .unwrap();
        assert_eq!(world.entries.len(), r.word() as usize, "case {case}");
        for entry in &world.entries {
            assert_eq!(entry.cell.mode, r.word());
            assert_eq!(entry.cell.only_for_shot, r.word() != 0);
            assert_eq!(entry.sector, r.optional());
            assert_eq!(entry.cell.planes.len(), r.word() as usize);
            for vector in entry
                .cell
                .planes
                .iter()
                .copied()
                .chain(entry.cell.vertices)
                .chain([
                    entry.cell.center,
                    entry.cell.bounds[0],
                    entry.cell.bounds[1],
                ])
            {
                for (a, b) in vector.into_iter().zip(r.vector()) {
                    assert_eq!(a.to_bits(), b.to_bits(), "case {case}: {a} vs {b}");
                }
            }
            assert_eq!(entry.logic.code as u32, r.word());
            assert_eq!(entry.logic.condition as u32, r.word());
            assert_eq!(entry.logic.value as u32, r.word());
            assert_eq!(entry.logic.output as u32, r.word());
            assert_eq!(entry.logic.events, [r.optional(), r.optional()]);
            for event in &entry.events {
                assert_eq!(event.is_some(), r.word() != 0);
                if let Some(event) = event {
                    assert_eq!(event.operator, r.word() as usize);
                    let v = r.vector();
                    assert_eq!(event.translate, v);
                    assert_eq!(v[3], 0.);
                    assert_eq!(event.cull_distance.to_bits(), r.word());
                }
            }
        }
    }
    assert!(r.0.is_empty());
}
