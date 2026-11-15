//! Original MonsterAI: activation, combat, movement, walking and event fields.
use super::{Parameters, monster_actor::Actor, monster_walk, shot};
use crate::{
    collision::{DynamicCell, movement::Collider, ray::Info, world::World},
    kx::Spline,
};
use std::{cell::RefCell, rc::Rc};
pub struct Player {
    pub position: [f32; 4],
    pub collider_position: [f32; 4],
    pub cell: Option<usize>,
    pub visibility_cell: Option<usize>,
}
pub struct Context<'a> {
    pub world: &'a mut World,
    pub dynamic: &'a [Rc<RefCell<DynamicCell>>],
    pub player_hit: Option<&'a Rc<RefCell<DynamicCell>>>,
    pub player: Player,
    pub parameters: &'a Parameters,
    pub switches: &'a mut [u8; 256],
    pub spline: Option<&'a Spline>,
    pub time: i32,
    pub time_reset: bool,
    pub machine_alive: bool,
    pub previous_alive: bool,
}
pub struct Fire {
    pub weapon: usize,
    pub origin: shot::Origin,
}
#[derive(Default)]
pub struct Actions {
    pub player_hits: Option<i32>,
    pub fire: Option<Fire>,
}
fn dot(v: [f32; 4]) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}
fn range(v: f32, max: f32, min: f32) -> f32 {
    if v >= max {
        max
    } else if v <= min {
        min
    } else {
        v
    }
}
impl Actor {
    pub fn think(&mut self, c: Context<'_>) -> Result<Actions, &'static str> {
        let mut actions = Actions::default();
        if self.health.life <= 0 {
            self.health.state = 1;
            self.health.life = 0;
            self.death.matrix = self.matrix;
            return Ok(actions);
        }
        let Some(player_cell) = c.player.cell else {
            return Ok(actions);
        };
        if self.health.state == 1 {
            let mut spawn = true;
            if self.parameters.flags & 1 != 0 && c.previous_alive {
                spawn = false;
            }
            if self.parameters.flags & 2 != 0 && !c.machine_alive {
                spawn = false;
            }
            if *c
                .switches
                .get(self.spawn_switch as usize)
                .ok_or("monster spawn switch out of range")?
                == 0
            {
                spawn = false;
            }
            if spawn {
                self.health.state = 2;
            }
        }
        let mut delta = [0.; 4];
        let mut distance = 0.;
        if self.health.state != 1 {
            delta = [
                self.body.position[0] - c.player.collider_position[0],
                0.,
                self.body.position[2] - c.player.collider_position[2],
                0.,
            ];
            distance = f64::from(dot(delta)).sqrt() as f32;
            if distance > 1e-10 {
                let scale = 1. / distance;
                for v in &mut delta[..3] {
                    *v *= scale;
                }
            } else {
                delta = [1., 0., 0., 0.];
            }
        }
        if self.health.state == 2 && distance < self.parameters.notice_radius {
            self.health.state = 3;
        }
        if (self.health.state == 3 || self.health.state == 4)
            && distance > self.parameters.notice_radius * 1.5
        {
            self.health.state = 2;
        }
        if self.health.state == 3 {
            let mut p0 = c.player.collider_position;
            let mut p1 = self.body.position;
            p0[1] += 1.5;
            p1[1] += 1.5;
            let mut cell = c
                .player
                .visibility_cell
                .ok_or("player visibility cell missing")?;
            let mut info = Info::default();
            if !c
                .world
                .collide_ray(&mut cell, p1, p0, &mut info, c.dynamic)?
            {
                self.health.state = 4;
            }
        }
        if self.health.state == 4 {
            self.melee_timer += 0.01 / self.parameters.melee_time;
            if self.melee_timer >= 1. {
                self.melee_timer = 1.;
            }
            self.ranged_timer += 0.01 / self.parameters.ranged_time;
            if self.ranged_timer >= 1. {
                self.ranged_timer = 1.;
            }
            let v = std::array::from_fn(|i| self.body.position[i] - c.player.position[i]);
            if dot(v) < 1.5 * 1.5 {
                if self.ranged_timer == 1.
                    && self.melee_timer == 1.
                    && self.parameters.melee_hits > 0
                {
                    self.melee_timer = 0.;
                    actions.player_hits = Some(self.parameters.melee_hits);
                }
            } else if self.ranged_timer == 1.
                && self.melee_timer == 1.
                && self.parameters.ranged_hits > 0
                && self.weapon_kind >= 0
            {
                self.ranged_timer = 0.;
                let mut p0 = c.player.collider_position;
                let mut p1 = self.body.position;
                p0[1] += 1.5;
                p1[1] += 1.5;
                let mut cell = player_cell;
                let mut info = Info::default();
                if !c
                    .world
                    .collide_ray(&mut cell, p1, p0, &mut info, c.dynamic)?
                {
                    for i in 0..4 {
                        p0[i] -= p1[i];
                    }
                    let scale = (1. / f64::from(dot(p0)).sqrt()) as f32;
                    for value in &mut p0[..3] {
                        *value *= scale;
                    }
                    actions.fire = Some(Fire {
                        weapon: self.weapon_kind as usize,
                        origin: shot::Origin::Monster {
                            id: self.id,
                            position: self.body.position,
                            direction: p0,
                            // Preserve a source null cell until the later death path.
                            cell: self.body.cell.unwrap_or(usize::MAX),
                        },
                    });
                }
            }
            distance -= if distance < self.parameters.charge_radius {
                self.parameters.min_radius
            } else {
                self.parameters.wait_radius
            };
            distance = range(distance, self.parameters.speed, -self.parameters.speed);
            let mut speed = delta.map(|v| v * -distance);
            if self.parameters.flags & 8 == 0
                && let Some(mut cell) = self.body.cell
            {
                let test_height =
                    c.parameters.stair_height + c.parameters.gravity + self.body.radius;
                let mut height = test_height;
                let mut p0 = self.body.position;
                for (value, axis) in p0[..3].iter_mut().zip(self.matrix[2]) {
                    *value += axis * self.body.radius;
                }
                let mut p1 = p0;
                p1[1] -= test_height;
                let v = std::array::from_fn::<_, 4, _>(|i| {
                    self.body.position[i] - self.body.old_position[i]
                });
                for (value, force) in speed[..3].iter_mut().zip(self.magnet_force) {
                    *value += force;
                }
                speed[1] = 0.;
                speed[3] = 0.;
                for i in 0..3 {
                    speed[i] += v[i] * 0.9;
                }
                self.hit_cell.borrow_mut().cell.mode |= 16;
                let mut info = Info {
                    flags: 2,
                    ..Info::default()
                };
                let hit = c.world.collide_ray(&mut cell, p1, p0, &mut info, c.dynamic);
                self.hit_cell.borrow_mut().cell.mode &= !16;
                if hit? {
                    height += info.distance;
                }
                let mut up = 0.;
                if height < test_height {
                    up += (test_height - height) * 0.1;
                } else {
                    speed[1] -= c.parameters.gravity;
                }
                self.body.old_position = self.body.position;
                self.body.old_position[1] += up;
                speed[1] += up;
                let mut collider = Collider {
                    position: self.body.position,
                    radius: self.body.radius,
                    cell: self.body.cell.unwrap(),
                };
                c.world
                    .move_collider(&mut collider, speed, 2, c.dynamic, c.player_hit)?;
                self.body.position = collider.position;
                self.body.cell = Some(collider.cell);
                self.matrix[0] = [-delta[2], 0., delta[0], 0.];
                self.matrix[1] = [0., 1., 0., 0.];
                self.matrix[2] = [-delta[0], 0., -delta[2], 0.];
                self.matrix[3] = self.body.position;
                let definition = monster_walk::DEFINITIONS
                    .get(self.walk_style)
                    .ok_or("monster walk style out of range")?;
                self.walk.advance(
                    definition,
                    &mut self.matrix,
                    monster_walk::Context {
                        world: c.world,
                        dynamic: c.dynamic,
                        spline: c.spline,
                        cell: self.body.cell,
                        time: c.time,
                        time_reset: c.time_reset,
                    },
                )?;
            }
            if self.body.cell.is_none() {
                self.health.hit(65536, c.switches)?;
            }
        }
        self.event.matrix = self.matrix;
        self.event.scale[0] = self.melee_timer - 0.5;
        if self.event.scale[0] < 0. {
            self.event.scale[0] += 1.;
        }
        self.event.scale[1] = self.ranged_timer - 0.5;
        if self.event.scale[1] < 0. {
            self.event.scale[1] += 1.;
        }
        self.event.scale[2] = self.health.life as f32 / self.parameters.life_max as f32;
        self.event.cull_distance = 35;
        Ok(actions)
    }
}
