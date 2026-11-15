//! Original player-build startup values and GetNewRoot decisions.
use super::{Inventory, Parameters, State};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub index: usize,
    pub quit: bool,
    pub restart: bool,
}
pub fn select(switches: &mut [u8; 256]) -> Selection {
    let game = switches[1];
    let index = if matches!(game, 0..=2) {
        2
    } else if game == 8 {
        0
    } else {
        1
    };
    if game == 3 {
        switches[1] = 6;
    }
    if game == 2 {
        switches[1] = 0;
    }
    Selection {
        index,
        quit: game == 3,
        restart: game == 2,
    }
}
pub fn switches() -> [u8; 256] {
    let mut switches = [0; 256];
    for (index, value) in [
        (0, 1),
        (11, 2),
        (8, 1),
        (9, 1),
        (10, 1),
        (13, 5),
        (14, 5),
        (15, 1),
    ] {
        switches[index] = value;
    }
    switches
}
impl State {
    pub fn player_build() -> Self {
        Self {
            inventory: Inventory {
                amounts: [100, 0, 0, 0, 100, 50, 0, 0],
                maxima: [100, 100, 100, 8, 500, 200, 100, 1000],
                weapons: [1, 1, 1, 0, 0, 0, 0, 0],
            },
            parameters: Parameters {
                gravity: 0.01,
                damp_ground: 0.05,
                damp_air: 0.05,
                damp_height: 0.05,
                stair_height: 0.55,
                eye_height: 1.6,
                mouse_turn: 0.01,
                mouse_look: 0.01,
                accel_side: 0.005,
                accel_forward: 0.005,
                accel_back: 0.005,
                jump_force: 0.35,
                damp_camera: 0.05,
                camera_offset: 1.6f32 - 0.55,
                ..Parameters::default()
            },
            ..State::default()
        }
    }
}
