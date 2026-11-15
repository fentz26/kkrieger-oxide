//! Player movement portion of the original OnTick time slice.
use super::Parameters;
use crate::{
    collision::{Cell, DynamicCell, movement::Collider, ray::Info, world::World},
    mesh::{Matrix, identity},
};
use std::{cell::RefCell, rc::Rc};

// Original fsincos evaluates before storing its two single-precision results.
fn orientation(angles: [f32; 3]) -> Matrix {
    let pairs = angles.map(|a| {
        let (s, c) = f64::from(a).sin_cos();
        (s as f32, c as f32)
    });
    let [(sx, cx), (sy, cy), (sz, cz)] = pairs;
    [
        [cy * cz, cy * sz, -sy, 0.],
        [-cx * sz + sx * sy * cz, cx * cz + sx * sy * sz, sx * cy, 0.],
        [sx * sz + cx * sy * cz, -sx * cz + cx * sy * sz, cx * cy, 0.],
        [0., 0., 0., 1.],
    ]
}

pub struct Motion {
    pub collider: Collider,
    pub old_position: [f32; 4],
    pub direction: f32,
    pub matrix: Matrix,
    pub accel_side: f32,
    pub accel_forward: f32,
    pub speed_up: f32,
    pub jump_timer: i32,
    pub on_ground: bool,
    pub off_ground_time: f32,
    pub step_foot: i32,
    pub step_timer: f32,
    pub hit_cell: Rc<RefCell<DynamicCell>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sound {
    pub slot: i32,
    pub volume: f32,
    pub near_distance: f32,
    pub far_distance: f32,
}

impl Motion {
    pub fn jump(&mut self, parameters: &Parameters) -> Option<Sound> {
        if self.on_ground && self.jump_timer == 0 {
            self.speed_up = parameters.jump_force;
            self.jump_timer = 10;
            Some(Sound {
                slot: 4,
                volume: 0.9,
                near_distance: 0.2,
                far_distance: 1.,
            })
        } else {
            None
        }
    }
    /// Original SetPlayer collision placement and hit-cell geometry.
    pub fn spawn(
        world: &World,
        mut position: [f32; 4],
        direction: f32,
    ) -> Result<Self, &'static str> {
        let cell = if let Some(cell) = world.find_cell(position) {
            cell
        } else {
            let &cell = world
                .adds
                .first()
                .ok_or("player world has no additive cell")?;
            for (i, value) in position[..3].iter_mut().enumerate() {
                *value = (world.entries[cell].cell.bounds[0][i]
                    + world.entries[cell].cell.bounds[1][i])
                    * 0.5;
            }
            cell
        };
        let mut matrix = identity();
        matrix[3] = [0., -0.25, 0., 1.];
        let hit = Cell::new(matrix, [0.75, 2.5, 0.75], 2);
        matrix[3] = position;
        matrix[3][1] += 0.85;
        Ok(Self {
            collider: Collider {
                position,
                radius: 0.5,
                cell,
            },
            old_position: position,
            direction,
            matrix: orientation([0., direction, 0.]),
            accel_side: 0.,
            accel_forward: 0.,
            speed_up: 0.,
            jump_timer: 0,
            on_ground: false,
            off_ground_time: 0.,
            step_foot: 0,
            step_timer: 0.,
            hit_cell: Rc::new(RefCell::new(DynamicCell::new(hit, matrix))),
        })
    }

    pub fn set_orientation(&mut self, look: f32, fly: bool) {
        self.matrix = orientation([if fly { look } else { 0. }, self.direction, 0.]);
    }

    /// Execute the movement prefix of one source slice. The caller subsequently
    /// runs Physic, Zones and the remaining simulation phases in source order.
    /// Returned sound is the original PlayerSound request; playback is separate.
    pub fn slice(
        &mut self,
        parameters: &Parameters,
        world: &mut World,
        slices: i32,
        dynamic: &[Rc<RefCell<DynamicCell>>],
    ) -> Result<Option<Sound>, &'static str> {
        if self.jump_timer > 0 {
            self.jump_timer -= 1;
        }
        let height = parameters.stair_height + parameters.gravity + self.collider.radius;
        let mut distance = height;
        let mut end = self.collider.position;
        end[1] -= height;
        let mut cell = self.collider.cell;
        let mut info = Info {
            flags: 1,
            ..Default::default()
        };
        if world.collide_ray(&mut cell, end, self.collider.position, &mut info, dynamic)? {
            distance += info.distance;
        }
        let mut speed: [f32; 4] =
            std::array::from_fn(|i| self.collider.position[i] - self.old_position[i]);
        let damp = 1.
            - if self.on_ground {
                parameters.damp_ground
            } else {
                parameters.damp_air
            };
        for (i, value) in speed[..3].iter_mut().enumerate() {
            *value *= damp;
            *value += self.matrix[0][i] * self.accel_side;
            *value += self.matrix[2][i] * self.accel_forward;
        }
        speed[1] += self.speed_up;
        self.speed_up = 0.;
        self.on_ground = false;
        let mut sound = None;
        let upforce = if distance < height {
            if self.off_ground_time >= 0.4 {
                sound = Some(Sound {
                    slot: 5,
                    volume: 1.,
                    near_distance: 0.2,
                    far_distance: 1.,
                });
                self.step_foot = 0;
                self.step_timer = 0.;
            }
            self.on_ground = true;
            self.off_ground_time = 0.;
            // Source declares upforce as sInt, truncating the float increment.
            ((height - distance) * 0.1) as i32
        } else {
            self.off_ground_time += slices as f32 * 0.01;
            speed[1] -= parameters.gravity;
            0
        };
        self.old_position = self.collider.position;
        self.old_position[1] += upforce as f32;
        speed[1] += upforce as f32;
        world.move_collider(&mut self.collider, speed, 1, dynamic, Some(&self.hit_cell))?;
        let mut hit = self.hit_cell.borrow_mut();
        hit.matrix1 = orientation([0., self.direction, 0.]);
        hit.matrix1[3] = self.collider.position;
        hit.matrix1[3][1] += 0.85;
        hit.matrix0[3] = hit.matrix1[3];
        Ok(sound)
    }
}
