use kkrieger_oxide::{
    collision::{
        Cell, connectivity,
        world::{Entry, Logic, World},
    },
    game::{input::Input, player::Motion},
    kx::Layout,
    mesh::identity,
};
use std::rc::Rc;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn word(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.0[..4].try_into().unwrap());
        self.0 = &self.0[4..];
        value
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vector(&mut self) -> [f32; 4] {
        std::array::from_fn(|_| self.float())
    }
    fn check(&mut self, values: impl IntoIterator<Item = f32>, case: usize, field: &str) {
        for (index, value) in values.into_iter().enumerate() {
            assert_eq!(
                value.to_bits(),
                self.word(),
                "case{case} {field}/{index}: {value}"
            );
        }
    }
}
#[test]
fn released_set_player_matches_native_world_pose_particles_and_hit_box() {
    let mut r = Reader(include_bytes!("fixtures/beta-player-placement.bin"));
    assert_eq!(r.word(), 64);
    for case in 0..64 {
        let count = r.word() as usize;
        let position = r.vector();
        let direction = r.float();
        let look = r.float();
        let mouse = [r.word() as i32, r.word() as i32];
        let old_matrix = std::array::from_fn(|_| r.vector());
        let entries: Vec<_> = (0..count)
            .map(|index| {
                let mut matrix = identity();
                matrix[3][0] = index as f32 * 2.;
                Entry {
                    cell: Cell::new(matrix, [8.; 3], 0),
                    sector: None,
                    logic: Logic {
                        code: 0,
                        condition: 0,
                        value: 0,
                        output: 0,
                        events: [None; 2],
                    },
                    events: [None, None],
                }
            })
            .collect();
        let world = World {
            entries,
            adds: (0..count).collect(),
            subs: vec![],
            zones: vec![],
            connectivity: connectivity::connect(&[]),
            faces: vec![None; count],
        };
        let mut motion =
            Motion::spawn_for_layout(&world, [0., 0., 0., 1.], 0., Layout::Beta2004).unwrap();
        motion.matrix = old_matrix;
        motion.speed_up = 0.2;
        motion.accel_forward = 0.3;
        motion.accel_side = -0.1;
        motion.on_ground = true;
        motion.step_timer = 0.4;
        motion.step_foot = 3;
        motion.off_ground_time = 0.5;
        let handle = Rc::clone(&motion.hit_cell);
        motion.place(&world, position, direction, look).unwrap();
        assert!(Rc::ptr_eq(&handle, &motion.hit_cell));
        r.check(motion.collider.position, case, "position");
        r.check([motion.look, motion.direction], case, "look/direction");
        let cell = (motion.collider.cell != usize::MAX).then_some(motion.collider.cell);
        let cell_token = cell.map_or(0, |id| 0x3020000 + id as u32 * 0x400);
        assert_eq!(cell_token, r.word(), "case{case} cell");
        let mut input = Input::new(123, [42, 43]);
        input.placement_mouse(mouse);
        for value in input.last_mouse {
            assert_eq!(value as u32, r.word());
        }
        assert_eq!(input.last_time, 123);
        r.check([motion.last_camera_y], case, "camera y");
        r.check(motion.camera_position, case, "camera position");
        r.check(
            [motion.speed_up, motion.accel_forward, motion.accel_side],
            case,
            "control",
        );
        assert_eq!(r.word(), 7); // untouched native Fly word, outside Motion
        assert_eq!(u32::from(motion.on_ground), r.word());
        r.check([motion.step_timer], case, "step timer");
        assert_eq!(motion.step_foot as u32, r.word());
        r.check([motion.off_ground_time], case, "off ground");
        r.check(
            motion.matrix.into_iter().flatten(),
            case,
            "retained player matrix",
        );
        let body = &motion.released_player.as_ref().unwrap().physics[0].0;
        let body_token = 0x30149fc;
        for (index, particle) in body.particles.iter().enumerate() {
            assert_eq!(particle.cell, cell);
            let mut raw = [0; 32];
            raw[..7].copy_from_slice(&particle.flags);
            raw[7] = cell_token;
            raw[8] = body_token + 0x348;
            raw[10] = particle.damping.to_bits();
            raw[11] = particle.weight.to_bits();
            raw[12..16].copy_from_slice(&particle.force.map(f32::to_bits));
            for (word, value) in raw[16..]
                .iter_mut()
                .zip(particle.vectors.into_iter().flatten())
            {
                *word = value.to_bits();
            }
            for (field, word) in raw.into_iter().enumerate() {
                assert_eq!(word, r.word(), "case{case} particle{index}/{field}");
            }
        }
        for constraint in &body.constraints {
            for word in [
                constraint.flags[0],
                constraint.flags[1],
                body_token + constraint.particles[0] as u32 * 128,
                body_token + constraint.particles[1] as u32 * 128,
                constraint.rest_length.to_bits(),
            ] {
                assert_eq!(word, r.word(), "case{case} constraint");
            }
        }
        let hit = motion.hit_cell.borrow();
        assert_eq!(hit.cell.mode, r.word(), "case{case} hit mode");
        assert_eq!(hit.cell.planes.len() as u32, r.word());
        r.check(
            hit.cell.planes.iter().flatten().copied(),
            case,
            "local planes",
        );
        r.check(hit.cell.center, case, "local center");
        r.check(
            hit.original_planes.iter().flatten().copied(),
            case,
            "original planes",
        );
        for matrix in [
            hit.confirmed_matrix,
            hit.current_matrix,
            hit.matrix0,
            hit.matrix1,
        ] {
            r.check(matrix.into_iter().flatten(), case, "hit matrix");
        }
        drop(hit);
        let scene = kkrieger_oxide::game::beta_monster::scene_cells::SceneCell::capture(
            Rc::clone(&motion.hit_cell),
            true,
        )
        .unwrap();
        r.check(
            scene.cell.pose.planes[..scene.cell.pose.count]
                .iter()
                .flatten()
                .copied(),
            case,
            "scene hit planes",
        );
        r.check(scene.cell.pose.center, case, "scene hit center");
    }
    assert!(r.0.is_empty());
}
