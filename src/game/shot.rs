//! Original ShotInfoTable and FireShot initialization.
use crate::{
    mesh::{Matrix, identity},
    random::Random,
};

#[derive(Clone, Copy, Debug)]
pub struct Definition {
    pub mode: i32,
    pub speed: f32,
    pub rotation: [f32; 3],
    pub position: [f32; 3],
    pub size: [f32; 3],
}
const fn definition(
    mode: i32,
    speed: f32,
    rx: f32,
    position: [f32; 3],
    size: [f32; 3],
) -> Definition {
    Definition {
        mode,
        speed,
        rotation: [rx, 0., 0.],
        position,
        size,
    }
}
pub const DEFINITIONS: [Definition; 8] = [
    definition(1, 0.250, -0.020, [0.575, -0.250, 1.250], [0.; 3]),
    definition(1, 0.200, -0.020, [0.585, -0.430, 1.250], [0.; 3]),
    definition(1, 0.1177, -0.020, [0.410, -0.135, 1.275], [0.; 3]),
    definition(0, 0., 0., [0.; 3], [0.; 3]),
    definition(2, 0.100, -0.020, [0.505, -0.135, 1.625], [0.5; 3]),
    definition(1, 0.250, 0., [0., 1.500, 1.000], [0.; 3]),
    definition(2, 0.100, -0.020, [0.505, -0.135, 1.625], [0.5; 3]),
    definition(1, 0.250, 0., [0., 1.935, 1.000], [0.; 3]),
];

pub enum Origin {
    Player {
        position: [f32; 4],
        look: f32,
        direction: f32,
        last_camera_y: f32,
        camera_offset: f32,
        cell: usize,
    },
    Monster {
        id: usize,
        position: [f32; 4],
        direction: [f32; 4],
        cell: usize,
    },
}

pub struct Spawn {
    pub weapon: usize,
    pub mode: i32,
    pub who: u32,
    pub double_kill: i32,
    pub cell: usize,
    pub old_position: [f32; 4],
    pub tensor: [f32; 4],
    pub matrix: Matrix,
    pub event_matrix: Matrix,
    pub event_operator: Option<usize>,
    pub event_monster: Option<usize>,
    pub event_active: bool,
}

fn dot(v: [f32; 4]) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}
fn unit(v: &mut [f32; 4]) {
    let e = dot(*v);
    if e > 1e-20 {
        let scale = (1. / f64::from(e).sqrt()) as f32;
        for x in &mut v[..3] {
            *x *= scale;
        }
    } else {
        *v = [1., 0., 0., 1.];
    }
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
        0.,
    ]
}
fn direction_matrix(v: [f32; 4]) -> Matrix {
    let mut m = identity();
    m[2] = v;
    unit(&mut m[2]);
    m[0] = cross(m[1], m[2]);
    unit(&mut m[0]);
    m[1] = cross(m[2], m[0]);
    unit(&mut m[1]);
    for axis in &mut m[..3] {
        axis[3] = 0.;
    }
    m
}

fn rotation(v: [f32; 4]) -> Matrix {
    let angle = f64::from(dot(v)).sqrt() as f32;
    if f64::from(angle) < 0.00000001 {
        return identity();
    }
    let (ss, cc) = (f64::from(angle).sin() as f32, f64::from(angle).cos() as f32);
    let mut u = v;
    unit(&mut u);
    let s = [
        [ss * 0., ss * u[2], ss * -u[1]],
        [ss * -u[2], ss * 0., ss * u[0]],
        [ss * u[1], ss * -u[0], ss * 0.],
    ];
    let mut mat = identity();
    for row in 0..3 {
        for col in 0..3 {
            let c = if row == col {
                cc * (1. - u[row] * u[row])
            } else {
                cc * -u[row] * u[col]
            };
            mat[row][col] = u[row] * u[col] + c + s[row][col];
        }
    }
    mat
}

pub enum Damage {
    Monster { monster: usize, hits: i32 },
    Player { hits: i32 },
}
pub struct Explosion {
    pub operator: Option<usize>,
    pub matrix: Matrix,
}
pub struct Outcome {
    pub alive: bool,
    pub damage: Option<Damage>,
    pub explosion: Option<Explosion>,
}

impl Spawn {
    /// Original ShotAI. Apply returned damage before queuing its explosion event.
    pub fn advance(
        &mut self,
        world: &mut crate::collision::world::World,
        dynamic: &[std::rc::Rc<std::cell::RefCell<crate::collision::DynamicCell>>],
        player_hit: Option<&std::rc::Rc<std::cell::RefCell<crate::collision::DynamicCell>>>,
        parameters: &super::Parameters,
        events: &super::Events,
    ) -> Result<Outcome, &'static str> {
        if self.weapon >= 8 {
            return Err("shot weapon out of range");
        }
        let mut info = crate::collision::ray::Info {
            zone_mask: 1 << (8 + self.weapon),
            flags: self.who,
            ..Default::default()
        };
        let mut speed = [0.; 4];
        for (i, v) in speed[..3].iter_mut().enumerate() {
            *v = self.matrix[3][i] - self.old_position[i];
            if self.mode == 2 {
                *v *= 0.999;
            }
        }
        let mut new_position = self.matrix[3];
        for i in 0..3 {
            new_position[i] += speed[i];
        }
        new_position[3] = 1.;
        if self.mode == 2 {
            new_position[1] -= 0.00025;
            let mat = rotation(self.tensor);
            for v in &mut self.tensor[..3] {
                *v *= 0.990;
            }
            let old = self.matrix;
            for (out, row) in self.matrix[..3].iter_mut().zip(&old[..3]) {
                for (col, value) in out[..3].iter_mut().enumerate() {
                    *value = (mat[0][col] * row[0] + mat[1][col] * row[1]) + mat[2][col] * row[2];
                }
            }
        }
        self.old_position = self.matrix[3];
        self.matrix[3] = new_position;
        if world.collide_ray(
            &mut self.cell,
            self.matrix[3],
            self.old_position,
            &mut info,
            dynamic,
        )? {
            if self.mode == 2 && self.double_kill == 0 {
                self.matrix[3] = info.position;
                for i in 0..3 {
                    self.matrix[3][i] += info.plane[i] * 0.125;
                }
                self.double_kill = 1;
                self.tensor = cross(self.tensor, info.plane);
                let dir = cross(speed, info.plane);
                for i in 0..3 {
                    self.tensor[i] += dir[i] * -0.5;
                    speed[i] *= 0.75;
                    self.old_position[i] = self.matrix[3][i] - speed[i];
                }
                let d =
                    speed[0] * info.plane[0] + speed[1] * info.plane[1] + speed[2] * info.plane[2];
                for i in 0..3 {
                    self.old_position[i] += info.plane[i] * (d * 2.);
                }
            } else {
                self.matrix[3] = info.position;
                let mut mat = direction_matrix(info.plane);
                mat[3] = self.matrix[3];
                return Ok(Outcome {
                    alive: false,
                    damage: None,
                    explosion: Some(Explosion {
                        operator: events.explode[0][self.weapon],
                        matrix: mat,
                    }),
                });
            }
        }
        self.double_kill = 0;
        if let Some(handle) = &info.dynamic_cell {
            let cell = handle.borrow();
            if let Some(monster) = cell.monster.filter(|_| self.who == 1) {
                return Ok(Outcome {
                    alive: false,
                    damage: Some(Damage::Monster {
                        monster,
                        hits: parameters.weapon_hits[self.weapon],
                    }),
                    explosion: Some(Explosion {
                        operator: events.explode[1][self.weapon],
                        matrix: self.matrix,
                    }),
                });
            }
            if self.who == 2 && player_hit.is_some_and(|player| std::rc::Rc::ptr_eq(player, handle))
            {
                return Ok(Outcome {
                    alive: false,
                    damage: Some(Damage::Player {
                        hits: parameters.weapon_hits[self.weapon],
                    }),
                    explosion: None,
                });
            }
        }
        Ok(Outcome {
            alive: true,
            damage: None,
            explosion: None,
        })
    }
}

pub fn spawn(
    weapon: usize,
    operator: Option<usize>,
    origin: Origin,
    rng: &mut Random,
) -> Result<Option<Spawn>, &'static str> {
    let info = DEFINITIONS.get(weapon).ok_or("shot weapon out of range")?;
    if info.mode == 0 {
        return Ok(None);
    }
    let (mat, who, cell, monster) = match origin {
        Origin::Player {
            position,
            look,
            direction,
            last_camera_y,
            camera_offset,
            cell,
        } => {
            let mut mat = super::player::orientation([
                look + info.rotation[0] * std::f32::consts::TAU,
                direction,
                0.,
            ]);
            let p = info.position;
            let rotated: [f32; 3] =
                std::array::from_fn(|i| (mat[0][i] * p[0] + mat[1][i] * p[1]) + mat[2][i] * p[2]);
            mat[3][..3].copy_from_slice(&rotated);
            mat[3][0] += position[0];
            mat[3][1] += last_camera_y + camera_offset;
            mat[3][2] += position[2];
            (mat, 1, cell, None)
        }
        Origin::Monster {
            id,
            position,
            direction,
            cell,
        } => {
            let mut mat = direction_matrix(direction);
            mat[3] = position;
            mat[3][1] += 0.75;
            (mat, 2, cell, Some(id))
        }
    };
    let speed = mat[2].map(|v| v * info.speed);
    let mut matrix = identity();
    matrix[3] = mat[3];
    let mut tensor = [0.; 4];
    if info.mode == 2 {
        loop {
            for v in &mut tensor[..3] {
                *v = rng.float() * 2. - 1.;
            }
            if dot(tensor) <= 1. {
                break;
            }
        }
        for v in &mut tensor[..3] {
            *v *= 0.01;
        }
    }
    Ok(Some(Spawn {
        weapon,
        mode: info.mode,
        who,
        double_kill: 0,
        cell,
        old_position: std::array::from_fn(|i| matrix[3][i] - speed[i]),
        tensor,
        matrix,
        event_matrix: mat,
        event_operator: operator,
        event_monster: monster,
        event_active: false,
    }))
}
