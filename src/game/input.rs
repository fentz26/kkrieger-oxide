//! Original player-build OnKey and OnTick mouse-look prefix.
use super::{
    State,
    player::{Motion, Sound},
};
use crate::collision::world::World;
pub const UP: u32 = 0x10000;
pub const DOWN: u32 = 0x10001;
pub const LEFT: u32 = 0x10002;
pub const RIGHT: u32 = 0x10003;
pub const ENTER: u32 = 10;
pub const BACKSPACE: u32 = 8;
pub const ESCAPE: u32 = 27;
pub const BREAK: u32 = 0x8000_0000;
pub const SHIFT_LEFT: u32 = 0x10021;
pub const CONTROL_RIGHT: u32 = 0x10025;
pub const MOUSE_LEFT: u32 = 0x10028;
pub struct Input {
    pub fly: bool,
    pub last_mouse: [i32; 2],
    pub last_time: i32,
    cheat: bool,
}
pub struct KeyResult {
    pub handled: bool,
    pub placed: bool,
    pub sounds: Vec<Sound>,
}
impl Input {
    pub fn new(time: i32, mouse: [i32; 2]) -> Self {
        Self {
            fly: false,
            last_mouse: mouse,
            last_time: time,
            cheat: false,
        }
    }
    /// SetPlayer refreshes counters without changing the tick clock.
    pub fn placement_mouse(&mut self, mouse: [i32; 2]) {
        self.last_mouse = mouse;
    }
    /// Call only after the original OnTick safety gate (valid cell and game-run
    /// switch). A long pause suppresses looking but still refreshes counters.
    pub fn look(
        &mut self,
        now: i32,
        mouse: [i32; 2],
        game: &State,
        motion: &mut Motion,
        switches: &[u8; 256],
    ) -> i32 {
        let time = now.wrapping_sub(self.last_time);
        self.last_time = self.last_time.wrapping_add(time);
        if time < 1000 {
            let f = game.parameters.mouse_turn * (f32::from(switches[13]) + 2.) / 7.;
            motion.direction += mouse[0].wrapping_sub(self.last_mouse[0]) as f32 * f;
            let f = game.parameters.mouse_look * (f32::from(switches[13]) + 2.) / 7.
                * (-f32::from(switches[12]) * 2. + 1.);
            motion.look += mouse[1].wrapping_sub(self.last_mouse[1]) as f32 * f;
            motion.look = motion
                .look
                .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        }
        self.last_mouse = mouse;
        motion.set_orientation(motion.look, self.fly);
        time
    }
    pub fn key(
        &mut self,
        key: u32,
        mouse: [i32; 2],
        game: &mut State,
        motion: &mut Motion,
        world: &World,
    ) -> Result<KeyResult, &'static str> {
        game.last_key = key & 0x8001ffff;
        let mut result = KeyResult {
            handled: false,
            placed: false,
            sounds: Vec::new(),
        };
        match game.last_key {
            32 => {
                if let Some(sound) = motion.jump(&game.parameters) {
                    result.sounds.push(sound);
                }
            }
            107 | 75 => {
                if let Some(sound) = game.inventory.hit(10) {
                    result.sounds.push(sound);
                }
            }
            119 | 87 => {
                motion.accel_forward = game.parameters.accel_forward;
                self.cheat = false;
                result.handled = true;
            }
            115 | 83 => {
                motion.accel_forward = -game.parameters.accel_back;
                self.cheat = false;
                result.handled = true;
            }
            100 | 68 => {
                motion.accel_side = game.parameters.accel_side;
                self.cheat = false;
                result.handled = true;
            }
            97 | 65 => {
                motion.accel_side = -game.parameters.accel_side;
                self.cheat = false;
                result.handled = true;
            }
            x if [119 | BREAK, 87 | BREAK, 115 | BREAK, 83 | BREAK].contains(&x) => {
                motion.accel_forward = 0.;
                self.cheat = false;
                result.handled = true;
            }
            x if [100 | BREAK, 68 | BREAK, 97 | BREAK, 65 | BREAK].contains(&x) => {
                motion.accel_side = 0.;
                self.cheat = false;
                result.handled = true;
            }
            121 | 89 => {
                self.fly = !self.fly;
                result.handled = true;
            }
            SHIFT_LEFT => {
                game.weapon.use_key = true;
                result.handled = true;
            }
            CONTROL_RIGHT | MOUSE_LEFT => {
                game.weapon.fire_key = true;
                result.handled = true;
            }
            x if x == CONTROL_RIGHT | BREAK || x == MOUSE_LEFT | BREAK => {
                game.weapon.fire_key = false;
                result.handled = true;
            }
            109 | 77 => {
                self.cheat = true;
                result.handled = true;
            }
            110 | 78 => {
                if self.cheat {
                    game.inventory.amounts = game.inventory.maxima;
                    game.inventory.weapons = [1; 8];
                }
                self.cheat = false;
                result.handled = true;
            }
            49..=57 => {
                if self.cheat {
                    if let Some(sound) = game.respawn_at((key & 15) as usize, motion, world)? {
                        self.placement_mouse(mouse);
                        result.placed = true;
                        result.sounds.push(sound);
                    }
                    self.cheat = false;
                } else {
                    let weapon = [-1, 0, 1, 2, 4, 6, -1, -1][(key & 7) as usize];
                    if weapon >= 0 && game.inventory.weapons[weapon as usize] != 0 {
                        game.weapon.next = weapon as usize;
                    }
                }
                result.handled = true;
            }
            _ => {}
        }
        Ok(result)
    }
}
