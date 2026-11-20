//! Player movement portion of the original OnTick time slice.
use super::Parameters;
use crate::{
    collision::{Cell, DynamicCell, movement::Collider, ray::Info, world::World},
    mesh::{Matrix, identity},
};
use std::{cell::RefCell, rc::Rc};

pub(super) use crate::mesh::euler_matrix_fsincos as orientation;

pub struct Motion {
    pub(super) document_layout: crate::kx::Layout,
    pub released_player: Option<super::beta_player::Player>,
    pub collider: Collider,
    pub old_position: [f32; 4],
    pub direction: f32,
    pub look: f32,
    pub last_camera_y: f32,
    pub camera_position: [f32; 4],
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
    /// Publish the main released particle after shared physics has finished.
    pub fn publish_released(&mut self, world: &World) -> Result<[f32; 4], &'static str> {
        let player = self
            .released_player
            .as_ref()
            .ok_or("released player body missing")?;
        let main = &player.physics[0].0.particles[0];
        let source = main
            .cell
            .map(|local| {
                world
                    .adds
                    .get(local)
                    .copied()
                    .ok_or("released player cell out of range")
            })
            .transpose()?;
        let position = main.vectors[0];
        let mut velocity = [0.; 4];
        for axis in 0..3 {
            velocity[axis] = super::beta_monster::down(super::beta_monster::add(
                f64::from(position[axis]),
                -f64::from(self.collider.position[axis]),
            ));
        }
        self.old_position = self.collider.position;
        self.collider.position = position;
        self.collider.cell = source.unwrap_or(usize::MAX);
        Ok(velocity)
    }
    /// Translate released particles before their shared physics pass.
    pub fn prepare_released(
        &mut self,
        parameters: &Parameters,
        slices: i32,
        clearance: f32,
    ) -> Result<Option<Sound>, &'static str> {
        let player = self
            .released_player
            .as_mut()
            .ok_or("released player body missing")?;
        if self.jump_timer > 0 {
            self.jump_timer -= 1;
        }
        let mut control = super::beta_player::Control {
            speed_up: self.speed_up,
            on_ground: self.on_ground,
            off_ground_time: self.off_ground_time,
            step_timer: self.step_timer,
            step_foot: self.step_foot,
        };
        let sound = control.prepare(
            &mut player.physics[0].0,
            &super::beta_player::Settings {
                right: self.matrix[0],
                forward: self.matrix[2],
                side: self.accel_side,
                ahead: self.accel_forward,
                gravity: parameters.gravity,
                height: parameters.stair_height,
                height_gain: parameters.damp_height,
                slices,
            },
            clearance,
        );
        self.speed_up = control.speed_up;
        self.on_ground = control.on_ground;
        self.off_ground_time = control.off_ground_time;
        self.step_timer = control.step_timer;
        self.step_foot = control.step_foot;
        let mut hit = self.hit_cell.borrow_mut();
        hit.matrix1 = orientation([0., self.direction, 0.]);
        hit.matrix1[3] = self.collider.position;
        hit.matrix1[3][1] = super::beta_monster::down(super::beta_monster::add(
            f64::from(hit.matrix1[3][1]),
            f64::from(0.85f32),
        ));
        hit.matrix0[3] = hit.matrix1[3];
        Ok(sound)
    }
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
    pub fn spawn(world: &World, position: [f32; 4], direction: f32) -> Result<Self, &'static str> {
        Self::spawn_for_layout(world, position, direction, crate::kx::Layout::Current)
    }
    pub fn spawn_for_layout(
        world: &World,
        mut position: [f32; 4],
        direction: f32,
        layout: crate::kx::Layout,
    ) -> Result<Self, &'static str> {
        let requested = position;
        let released = layout == crate::kx::Layout::Beta2004;
        let found = if released {
            world.adds.iter().copied().find(|&id| {
                use super::beta_monster::hull::{Hull, outside_mask};
                let source = &world.entries[id].cell;
                let hull = Hull::from_vertices(source.vertices, source.mode);
                outside_mask(&hull.planes[..hull.count], position, 0.) == 0
            })
        } else {
            world.find_cell(position)
        };
        let cell = if let Some(cell) = found {
            cell
        } else if let Some(&cell) = world.adds.first().filter(|_| !released) {
            for (i, value) in position[..3].iter_mut().enumerate() {
                *value = (world.entries[cell].cell.bounds[0][i]
                    + world.entries[cell].cell.bounds[1][i])
                    * 0.5;
            }
            cell
        } else {
            // Original SetPlayer keeps a null cell for menu/intro scenes.
            usize::MAX
        };
        let mut matrix = identity();
        let hit = if released {
            // Released SetPlayer -> PartBox::Init constructs an unshifted
            // 0.5 x 1.5 x 0.5 box in mode 1, using the released hull arithmetic.
            let mut hit = Cell::new(matrix, [0.5, 1.5, 0.5], 1);
            let hull = super::beta_monster::hull::Hull::from_vertices(hit.vertices, 1);
            hit.planes = hull.planes[..hull.count].to_vec();
            hit.bounds = hull.bounds;
            hit.center = hull.center;
            hit
        } else {
            matrix[3] = [0., -0.25, 0., 1.];
            Cell::new(matrix, [0.75, 2.5, 0.75], 2)
        };
        matrix[3] = position;
        matrix[3][1] = if released {
            super::beta_monster::down(super::beta_monster::add(
                f64::from(position[1]),
                f64::from(0.85f32),
            ))
        } else {
            matrix[3][1] + 0.85
        };
        Ok(Self {
            document_layout: layout,
            released_player: released.then(|| {
                super::beta_player::Player::new(
                    position,
                    found.and_then(|source| world.adds.iter().position(|&id| id == source)),
                )
            }),
            collider: Collider {
                position,
                radius: 0.5,
                cell,
            },
            old_position: position,
            direction,
            look: 0.,
            last_camera_y: requested[1],
            camera_position: requested,
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

    /// Collision/camera portion of SetPlayer. The input owner refreshes mouse
    /// counters separately. Preserve motion timers and the stable hit-cell handle.
    pub fn place(
        &mut self,
        world: &World,
        position: [f32; 4],
        direction: f32,
        look: f32,
    ) -> Result<(), &'static str> {
        let fresh = Self::spawn_for_layout(world, position, direction, self.document_layout)?;
        self.collider = fresh.collider;
        self.released_player = fresh.released_player;
        self.old_position = fresh.old_position;
        self.direction = direction;
        self.look = look;
        self.last_camera_y = fresh.last_camera_y;
        self.camera_position = fresh.camera_position;
        *self.hit_cell.borrow_mut() = fresh.hit_cell.borrow().clone();
        Ok(())
    }

    /// Camera smoothing runs after Physic and Zones for every movement slice.
    pub fn update_camera(&mut self, parameters: &Parameters) {
        if self.document_layout == crate::kx::Layout::Beta2004 {
            use super::beta_monster::{add, down, mul};
            let previous = f64::from(self.last_camera_y);
            let delta = add(f64::from(self.collider.position[1]), -previous);
            let smooth = add(previous, mul(delta, f64::from(parameters.damp_camera)));
            self.last_camera_y = down(smooth);
            self.camera_position = self.collider.position;
            // Native FST stores LastCameraY without discarding the x87 value.
            // Keep its wide exponent for the subsequent camera-offset addition.
            self.camera_position[1] = down(add(smooth, f64::from(parameters.camera_offset)));
            return;
        }
        self.last_camera_y +=
            (self.collider.position[1] - self.last_camera_y) * parameters.damp_camera;
        self.camera_position = self.collider.position;
        self.camera_position[1] = self.last_camera_y;
        self.camera_position[1] += parameters.camera_offset;
    }
    /// Original GetCamera space; projection zoom is one on both axes.
    pub fn camera(&self, zoom_position: bool) -> Matrix {
        if self.document_layout == crate::kx::Layout::Beta2004 {
            use super::beta_monster::{add, down, mul};
            let look = if zoom_position { 0. } else { self.look };
            let mut matrix = released_orientation([look, self.direction, 0.]);
            matrix[3] = if zoom_position {
                self.collider.position
            } else {
                self.camera_position
            };
            matrix[3][3] = 1.;
            if zoom_position {
                for axis in 0..3 {
                    let back = mul(-2., f64::from(matrix[2][axis]));
                    matrix[3][axis] = down(add(back, f64::from(matrix[3][axis])));
                }
            }
            return matrix;
        }
        let mut matrix = orientation([
            if zoom_position { 0. } else { self.look },
            self.direction,
            0.,
        ]);
        matrix[3] = if zoom_position {
            self.collider.position
        } else {
            self.camera_position
        };
        matrix[3][3] = 1.;
        if zoom_position {
            for axis in 0..3 {
                matrix[3][axis] += matrix[2][axis] * -2.;
            }
        }
        matrix
    }

    pub fn set_orientation(&mut self, look: f32, fly: bool) {
        self.look = look;
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
        self.slice_with_velocity(parameters, world, slices, dynamic)
            .map(|(sound, _)| sound)
    }
    pub fn slice_with_velocity(
        &mut self,
        parameters: &Parameters,
        world: &mut World,
        slices: i32,
        dynamic: &[Rc<RefCell<DynamicCell>>],
    ) -> Result<(Option<Sound>, [f32; 4]), &'static str> {
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
        Ok((sound, speed))
    }
}

/// Released InitEuler at `0x805a9d`: separate fsin/fcos results stay on the
/// x87 stack under PC=24/RC=down; only cos(y), cos(z) and sin(z)*sin(y) are
/// reloaded from single-precision spills.
fn released_orientation(angles: [f32; 3]) -> Matrix {
    use super::beta_monster::{add, down, mul};
    let [(sx, cx), (sy, cy), (sz, cz)] = angles.map(|a| libm::sincos(f64::from(a)));
    let cy = f64::from(down(cy));
    let cz_spill = f64::from(down(cz));
    let czsy = mul(cz, sy);
    let szsy = mul(sz, sy);
    let szsy_spill = f64::from(down(szsy));
    [
        [down(mul(cy, cz)), down(mul(sz, cy)), down(-sy), 0.],
        [
            down(add(mul(czsy, sx), -mul(sz, cx))),
            down(add(mul(szsy, sx), mul(cz_spill, cx))),
            down(mul(cy, sx)),
            0.,
        ],
        [
            down(add(mul(czsy, cx), mul(sz, sx))),
            down(add(mul(szsy_spill, cx), -mul(cz_spill, sx))),
            down(mul(cy, cx)),
            0.,
        ],
        [0., 0., 0., 1.],
    ]
}
