//! Original weapon animation and firing decisions from OnTick.
use super::{
    Events, Inventory, Parameters,
    player::{Motion, Sound},
};

pub struct State {
    pub current: usize,
    pub next: usize,
    pub fire_key: bool,
    pub use_key: bool,
    pub cooldown: f32,
    pub timer: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            current: 1,
            next: 0,
            fire_key: false,
            use_key: false,
            cooldown: 0.,
            timer: 0.,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Fire {
    pub weapon: usize,
    pub operator: usize,
}

impl State {
    pub fn animate(&mut self, slices: i32, accel_forward: f32) {
        if self.next != self.current {
            if self.timer < 0.75 {
                self.timer += slices as f32 * 0.01 * 5.;
            }
            self.timer += slices as f32 * 0.01;
        } else if self.timer < 0.25 {
            self.timer += slices as f32 * 0.01;
            if self.timer > 0.25 {
                self.timer = 0.25;
            }
        } else if self.timer > 0.251 || accel_forward > 0. {
            self.timer += slices as f32 * 0.003;
            if self.timer > 0.75 {
                if accel_forward > 0. {
                    self.timer -= 0.5;
                    if self.timer > 0.75 {
                        self.timer = 0.25;
                    }
                } else {
                    self.timer = 0.25;
                }
            }
        }
    }

    /// Logical part of AddEvents switching; caller recreates WeaponEvent when true.
    pub fn finish_switch(&mut self) -> bool {
        if self.next != self.current && self.timer >= 1. {
            self.current = self.next;
            self.timer = 0.;
            true
        } else {
            false
        }
    }

    /// Returns the original FireShot invocation and optional empty-ammo sound.
    /// Projectile creation remains the caller's phase.
    pub fn fire(
        &mut self,
        inventory: &mut Inventory,
        parameters: &Parameters,
        events: &Events,
        slices: i32,
    ) -> Result<(Option<Fire>, Option<Sound>), &'static str> {
        if self.current >= 8 || self.next >= 8 {
            return Err("weapon index out of range");
        }
        let mut sound = None;
        let mut shot = None;
        if self.fire_key {
            let ammo = &mut inventory.amounts[4 + self.current / 2];
            if *ammo == 0 && self.current != 0 {
                sound = Some(Sound {
                    slot: 15,
                    volume: 0.8,
                    near_distance: 0.2,
                    far_distance: 1.,
                });
            }
            if (*ammo > 0 || self.current == 0)
                && self.current == self.next
                && self.cooldown <= 0.
                && self.timer >= 0.25
            {
                if self.current != 0 {
                    *ammo = ammo.wrapping_sub(1);
                }
                self.cooldown = parameters.weapon_cool[self.current];
                if parameters.weapon_flags[self.current] & 1 == 0 {
                    self.fire_key = false;
                }
                if let Some(operator) = events.shot[self.current] {
                    shot = Some(Fire {
                        weapon: self.current,
                        operator,
                    });
                }
                if parameters.weapon_flags[self.current] & 1 == 0 {
                    self.fire_key = false;
                }
            }
        }
        if self.cooldown >= 0. {
            self.cooldown -= slices as f32 * 0.01;
        }
        Ok((shot, sound))
    }
}

pub fn step(motion: &mut Motion, slices: i32) -> Option<Sound> {
    if motion.accel_forward != 0. || motion.accel_side != 0. {
        motion.step_timer += slices as f32 * 0.01;
        if motion.step_timer > 0.5 && motion.on_ground {
            motion.step_timer -= 0.5;
            motion.step_foot = 1 - motion.step_foot;
            return Some(Sound {
                slot: motion.step_foot,
                volume: 0.8,
                near_distance: 0.2,
                far_distance: 1.,
            });
        }
    }
    None
}
