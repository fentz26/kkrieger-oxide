//! Original zone logic phase. Event-instance resets are returned in zone order.
use super::{State, player::Motion};
use crate::collision::{DynamicCell, world::World};
use std::{cell::RefCell, rc::Rc};

pub struct Runtime {
    /// Respawn countdown for each entry in World::zones, in source order.
    pub respawn: Vec<i32>,
    released: bool,
}
impl Runtime {
    pub fn new(world: &World) -> Self {
        Self::new_for_layout(world, crate::kx::Layout::Current)
    }
    pub fn new_for_layout(world: &World, layout: crate::kx::Layout) -> Self {
        Self {
            respawn: vec![0; world.zones.len()],
            released: layout == crate::kx::Layout::Beta2004,
        }
    }
    /// Each returned world entry needs its second event's instance chain cleared.
    pub fn tick(
        &mut self,
        game: &mut State,
        motion: &mut Motion,
        world: &mut World,
        dynamic: &[Rc<RefCell<DynamicCell>>],
        switches: &mut [u8; 256],
    ) -> Result<Vec<usize>, &'static str> {
        if self.respawn.len() != world.zones.len() {
            return Err("zone runtime does not match world");
        }
        let mut resets = Vec::new();
        for (index, &entry) in world.zones.iter().enumerate() {
            let zone = &mut world.entries[entry];
            let timer = &mut self.respawn[index];
            if *timer > 0 && *timer < 0x40000000 {
                if *timer <= 10 {
                    *timer = 0;
                    if !self.released {
                        resets.push(entry);
                    }
                } else {
                    *timer -= 10;
                }
            }
            let logic = &zone.logic;
            if switches[logic.condition as usize] != 0 {
                let hit = zone.cell.interactions;
                let action = match logic.code & 0xf0 {
                    0x10 => hit.enter & 2 != 0,
                    0x20 => hit.leave & 2 != 0,
                    0x30..=0x70 => {
                        if hit.collided & 2 != 0 && *timer == 0 {
                            *timer = match logic.code & 0xf0 {
                                0x30 => i32::MAX,
                                0x40 => 10000,
                                0x50 => 30000,
                                0x60 => 60000,
                                _ => 300000,
                            };
                            true
                        } else {
                            false
                        }
                    }
                    0x80 => hit.collided & 2 != 0 && game.weapon.use_key,
                    0x90 => hit.enter & 0xff00 != 0,
                    0xa0 => true,
                    0xb0 => hit.collided & 2 != 0,
                    0xc0 => hit.enter & 0xff02 != 0,
                    _ => false,
                };
                let out = logic.output as usize;
                let slot = out & 15;
                match logic.code & 15 {
                    1 if action => switches[out] = 1,
                    2 if action => switches[out] = 0,
                    3 if action => switches[out] = u8::from(switches[out] == 0),
                    4 => switches[out] = u8::from(action),
                    5..=9 if action => {
                        let amount = if slot < 8 {
                            &mut game.inventory.amounts[slot]
                        } else {
                            &mut game.inventory.weapons[slot - 8]
                        };
                        *amount = match logic.code & 15 {
                            5 => logic.value,
                            6 => (*amount).max(logic.value),
                            7 => (*amount).min(logic.value),
                            8 => {
                                if slot >= 8 && (!self.released || *amount == 0) {
                                    game.weapon.next = slot - 8;
                                }
                                let max = if slot < 8 {
                                    game.inventory.maxima[slot]
                                } else {
                                    1
                                };
                                amount.wrapping_add(logic.value).min(max)
                            }
                            _ => amount.wrapping_sub(logic.value).max(0),
                        };
                    }
                    10 if action => {
                        motion.speed_up = if self.released {
                            use super::beta_monster::{add, down, mul};
                            down(add(
                                f64::from(motion.speed_up),
                                mul(f64::from(logic.value), f64::from(0.001f32)),
                            ))
                        } else {
                            motion.speed_up + 0.001 * logic.value as f32
                        };
                    }
                    11 if action => switches[32 + (logic.value & 15) as usize] = 1,
                    _ => {}
                }
            }
            zone.cell.interactions = Default::default();
        }
        for cell in dynamic {
            cell.borrow_mut().cell.interactions = Default::default();
        }
        Ok(resets)
    }
}
