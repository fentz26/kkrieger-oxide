use kkrieger_oxide::{
    collision::{
        Cell, connectivity, faces,
        world::{Entry, Logic, World},
    },
    game::{Parameters, player::Motion},
};
use std::rc::Rc;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        v
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn cell(&mut self) -> Cell {
        let mode = self.word();
        Cell::from_vertices(std::array::from_fn(|_| self.vector()), mode)
    }
    fn check_vector(&mut self, v: [f32; 4], case: usize, step: usize) {
        for (a, b) in v.into_iter().zip(self.vector()) {
            assert_eq!(a.to_bits(), b.to_bits(), "{case}/{step}: {a} vs {b}");
        }
    }
}
#[test]
fn original_player_movement_slice_preserves_grounding_history_and_hit_cell() {
    let mut r = Reader(include_bytes!("fixtures/player-motion.bin"));
    assert_eq!(r.word(), 64);
    assert_eq!(r.word(), 12);
    let mut landings = 0;
    for case in 0..64 {
        let count = r.word();
        let cells: Vec<_> = (0..count).map(|_| r.cell()).collect();
        let graph = connectivity::connect(&cells);
        let cell_faces = cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if c.mode == 0 {
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
        let parameters = Parameters {
            gravity: r.float(),
            damp_ground: r.float(),
            damp_air: r.float(),
            stair_height: r.float(),
            ..Default::default()
        };
        let mut motion = Motion::spawn(&world, r.vector(), 0.).unwrap();
        motion.jump_timer = r.word() as i32;
        motion.on_ground = r.word() != 0;
        motion.off_ground_time = r.float();
        motion.step_foot = r.word() as i32;
        motion.step_timer = r.float();
        let dynamic = [Rc::clone(&motion.hit_cell)];
        for step in 0..12 {
            let slices = r.word() as i32;
            motion.direction = r.float();
            motion.set_orientation(0., false);
            motion.accel_side = r.float();
            motion.accel_forward = r.float();
            motion.speed_up = r.float();
            let sound = motion
                .slice(&parameters, &mut world, slices, &dynamic)
                .unwrap();
            r.check_vector(motion.collider.position, case, step);
            r.check_vector(motion.old_position, case, step);
            assert_eq!(motion.collider.cell, r.word() as usize);
            assert_eq!(motion.collider.radius.to_bits(), r.word());
            assert_eq!(motion.jump_timer, r.word() as i32);
            assert_eq!(motion.on_ground, r.word() != 0);
            assert_eq!(motion.off_ground_time.to_bits(), r.word());
            assert_eq!(motion.step_foot, r.word() as i32);
            assert_eq!(motion.step_timer.to_bits(), r.word());
            assert_eq!(motion.speed_up.to_bits(), r.word());
            assert_eq!(u32::from(sound.is_some()), r.word());
            if let Some(sound) = sound {
                assert_eq!(sound.slot, 5);
                assert_eq!(sound.volume, 1.);
                landings += 1;
            }
            let hit = motion.hit_cell.borrow();
            for matrix in [hit.matrix0, hit.matrix1] {
                for v in matrix {
                    r.check_vector(v, case, step);
                }
            }
            for &i in &world.connectivity.sorted {
                let state = world.entries[i].cell.interactions;
                assert_eq!(
                    [state.enter, state.leave, state.collided],
                    [r.word(), r.word(), r.word()]
                );
            }
            let state = hit.cell.interactions;
            assert_eq!(
                [state.enter, state.leave, state.collided],
                [r.word(), r.word(), r.word()]
            );
        }
    }
    assert!(landings > 0);
    assert!(r.0.is_empty());
}
