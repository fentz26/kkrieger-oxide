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
    switches_for_layout(crate::kx::Layout::Current)
}
/// Released constructor has one intro switch and no development option defaults.
pub fn switches_for_layout(layout: crate::kx::Layout) -> [u8; 256] {
    if layout == crate::kx::Layout::Beta2004 {
        let mut switches = [0; 256];
        switches[0] = 1;
        switches[1] = 8;
        return switches;
    }
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
/// The released player explicitly places the origin after collision-world setup.
pub fn initial_player_position(parameters: &Parameters, layout: crate::kx::Layout) -> [f32; 4] {
    if layout == crate::kx::Layout::Beta2004 {
        [0.; 4]
    } else {
        parameters.start_position
    }
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

/// Original SetPainter: unwrap DEMO/IPP first inputs until a scene cache is
/// reached. A mesh or an empty chain leaves the previous collision world intact.
pub fn painter_scene(
    doc: &crate::kx::Document<'_>,
    root: usize,
) -> Result<Option<usize>, crate::scene::Error> {
    let mut current = root;
    for _ in 0..=doc.operators.len() {
        if let Some(scene) = crate::scene::cache_scene_identity(doc, current)? {
            return Ok(Some(scene));
        }
        if crate::scene::presentation_kind(doc, current)? == crate::scene::PresentationKind::Other {
            return Ok(None);
        }
        let Some(&next) = doc.operators[current].inputs.first() else {
            return Ok(None);
        };
        current = next;
    }
    Err(crate::scene::Error {
        operator: root,
        reason: "painter input cycle",
    })
}
