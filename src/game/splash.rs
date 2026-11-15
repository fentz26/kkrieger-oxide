//! Original SplashCalc and the OnTick splash-damage phase.
use super::{State, player::Sound};
use crate::collision::{DynamicCell, ray::Info, world::World};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub struct Splash {
    pub enabled: bool,
    pub position: [f32; 4],
    pub range: f32,
    pub hits: i32,
}
pub struct Targets<'a> {
    pub monsters: &'a [[f32; 4]],
    pub player: [f32; 4],
}
impl Splash {
    pub fn trigger(
        &mut self,
        previous: &mut bool,
        hits: i32,
        range: f32,
        enable: f32,
        position: [f32; 4],
    ) {
        let active = enable > 0.;
        if active && !*previous {
            *self = Self {
                enabled: true,
                hits,
                range,
                position,
            };
        }
        *previous = active;
    }
    pub fn calculate(
        &self,
        target: [f32; 4],
        cell: Option<usize>,
        world: &mut World,
        dynamic: &[Rc<RefCell<DynamicCell>>],
    ) -> Result<i32, &'static str> {
        let d: [f32; 3] = std::array::from_fn(|i| target[i] - self.position[i]);
        let mut distance = f64::from(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() as f32;
        if distance < self.range {
            let mut end = target;
            end[1] += 0.5;
            let blocked = if let Some(mut cell) = cell {
                world.collide_ray(&mut cell, end, self.position, &mut Info::default(), dynamic)?
            } else {
                false
            };
            if !blocked {
                distance = 1. - distance / self.range;
                return Ok((self.hits as f32 * distance) as i32);
            }
        }
        Ok(0)
    }
}
impl State {
    pub fn apply_splash(
        &mut self,
        splash: &mut Splash,
        targets: Targets<'_>,
        world: &mut World,
        dynamic: &[Rc<RefCell<DynamicCell>>],
        switches: &mut [u8; 256],
    ) -> Result<Option<Sound>, &'static str> {
        if !splash.enabled {
            return Ok(None);
        }
        if targets.monsters.len() != self.monsters.len() {
            return Err("splash monster positions do not match registry");
        }
        let cell = world.find_cell(splash.position);
        for (monster, &position) in self.monsters.iter_mut().zip(targets.monsters) {
            let hits = splash.calculate(position, cell, world, dynamic)?;
            monster.hit(hits, switches)?;
        }
        splash.hits /= 2;
        let sound = self
            .inventory
            .hit(splash.calculate(targets.player, cell, world, dynamic)?);
        splash.enabled = false;
        Ok(sound)
    }
}
