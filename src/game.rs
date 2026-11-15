//! Original gameplay state and operator parameter application.
pub mod player;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inventory {
    /// Original contiguous collectables: life, armor, alpha, beta, four ammo kinds.
    pub amounts: [i32; 8],
    pub maxima: [i32; 8],
    pub weapons: [i32; 8],
}
impl Inventory {
    /// Original KKriegerPlayer::Hit. Armor changes the damage calculation but
    /// is not consumed; the returned request is the source hurt sound.
    pub fn hit(&mut self, hits: i32) -> Option<player::Sound> {
        let armor = self.amounts[1];
        let damage = if hits > armor {
            hits.wrapping_sub(armor).wrapping_add(armor / 4)
        } else {
            hits / 4
        };
        self.amounts[0] = self.amounts[0].wrapping_sub(damage).max(0);
        (hits > 4).then_some(player::Sound {
            slot: 10,
            volume: 1.,
            near_distance: 0.2,
            far_distance: 160.,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Parameters {
    pub gravity: f32,
    pub damp_ground: f32,
    pub damp_air: f32,
    pub damp_height: f32,
    pub stair_height: f32,
    pub eye_height: f32,
    pub mouse_turn: f32,
    pub mouse_look: f32,
    pub accel_side: f32,
    pub accel_forward: f32,
    pub accel_back: f32,
    pub jump_force: f32,
    pub start_position: [f32; 4],
    pub damp_camera: f32,
    pub weapon_hits: [i32; 8],
    pub weapon_cool: [f32; 8],
    pub weapon_flags: [i32; 8],
    pub camera_offset: f32,
}

#[derive(Default)]
pub struct Events {
    pub shot: [Option<usize>; 8],
    pub optics: [Option<usize>; 8],
    pub explode: [[Option<usize>; 8]; 2],
}
impl Events {
    pub fn apply(&mut self, kind: u32, links: &[Option<usize>]) -> Result<(), &'static str> {
        let links: [Option<usize>; 8] = links
            .get(..8)
            .ok_or("truncated weapon event links")?
            .try_into()
            .unwrap();
        match kind {
            1 => self.optics = links,
            2 => self.explode[0] = links,
            3 => self.explode[1] = links,
            _ => self.shot = links,
        }
        Ok(())
    }
}

pub struct State {
    pub inventory: Inventory,
    pub parameters: Parameters,
    pub events: Events,
    pub reset_by_operator: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            inventory: Inventory::default(),
            parameters: Parameters::default(),
            events: Events::default(),
            reset_by_operator: true,
        }
    }
}
impl State {
    /// Original Exec_KKrieger_Para: maxima and tuning update every call; initial
    /// collectables and weapon ownership update only after a game reset.
    pub fn apply_parameters(&mut self, words: &[u32]) -> Result<(), &'static str> {
        if words.len() < 57 {
            return Err("truncated gameplay parameters");
        }
        for i in 0..8 {
            self.inventory.maxima[i] = words[i * 2 + 1] as i32;
        }
        if self.reset_by_operator {
            self.reset_by_operator = false;
            for i in 0..8 {
                self.inventory.amounts[i] = words[i * 2] as i32;
                self.inventory.weapons[i] = ((words[16] >> i) & 1) as i32;
            }
        }
        let f = |i| f32::from_bits(words[i]);
        self.parameters = Parameters {
            gravity: f(17),
            damp_ground: f(18),
            damp_air: f(19),
            damp_height: f(20),
            stair_height: f(21),
            eye_height: f(22),
            mouse_turn: f(23),
            mouse_look: f(24),
            accel_side: f(25),
            accel_forward: f(26),
            accel_back: f(27),
            jump_force: f(28),
            start_position: [f(29), f(30), f(31), 1.],
            damp_camera: f(56),
            weapon_hits: std::array::from_fn(|i| words[32 + i] as i32),
            weapon_cool: std::array::from_fn(|i| f(40 + i)),
            weapon_flags: std::array::from_fn(|i| words[48 + i] as i32),
            camera_offset: f(22) - f(21),
        };
        Ok(())
    }
}
