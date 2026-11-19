//! Released player input translation before the shared particle physics pass.
use super::{
    beta_monster::{add, body::Body, down, mul, runtime::BodyRegistration},
    player::Sound,
};

/// Persistent player particles, registered before scene actor bodies.
pub struct Player {
    pub physics: [(Body, BodyRegistration); 1],
}
impl Player {
    pub fn new(position: [f32; 4], cell: Option<usize>) -> Self {
        Self {
            physics: [(
                Body::new(position, cell, 1),
                BodyRegistration {
                    particles: [0, 1, 2, 3, 4],
                    skip_cell: None,
                },
            )],
        }
    }
}

pub struct Settings {
    pub right: [f32; 4],
    pub forward: [f32; 4],
    pub side: f32,
    pub ahead: f32,
    pub gravity: f32,
    pub height: f32,
    pub height_gain: f32,
    pub slices: i32,
}
#[derive(Clone, Debug, Default)]
pub struct Control {
    pub speed_up: f32,
    pub on_ground: bool,
    pub off_ground_time: f32,
    pub step_timer: f32,
    pub step_foot: i32,
}
impl Control {
    /// `clearance` is the downward cast distance plus gravity and height.
    pub fn prepare(
        &mut self,
        body: &mut Body,
        settings: &Settings,
        clearance: f32,
    ) -> Option<Sound> {
        let mut delta = [0.; 4];
        for (axis, value) in delta[..3].iter_mut().enumerate() {
            *value = down(add(
                mul(f64::from(settings.side), f64::from(settings.right[axis])),
                0.,
            ));
            *value = down(add(
                mul(f64::from(settings.ahead), f64::from(settings.forward[axis])),
                f64::from(*value),
            ));
        }
        delta[1] = down(add(f64::from(delta[1]), f64::from(self.speed_up)));
        self.speed_up = 0.;
        self.on_ground = false;
        let mut sound = None;
        if add(f64::from(settings.gravity), f64::from(settings.height)) > f64::from(clearance) {
            if self.off_ground_time >= 0.4 {
                sound = Some(Sound {
                    slot: 5,
                    volume: 1.,
                    near_distance: 0.2,
                    far_distance: 160.,
                });
                self.step_timer = 0.;
                self.step_foot = 0;
            }
            self.on_ground = true;
            let correction = add(f64::from(settings.height), -f64::from(settings.gravity));
            let correction = add(correction, -f64::from(clearance));
            let correction = mul(correction, f64::from(settings.height_gain));
            let correction = add(correction, f64::from(settings.gravity));
            delta[1] = down(add(correction, f64::from(delta[1])));
            self.off_ground_time = 0.;
        } else {
            self.off_ground_time = down(add(
                mul(f64::from(settings.slices), f64::from(0.01f32)),
                f64::from(self.off_ground_time),
            ));
        }
        body.translate(delta);
        sound
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickDispatch {
    Idle,
    Run,
    Restart,
    Quit,
}
/// Released OnTick prologue, before input, physics and scene execution.
pub fn dispatch_tick(
    player_cell: bool,
    particle_cell: bool,
    mode: u8,
    counter: &mut i32,
    slices: i32,
) -> TickDispatch {
    if !player_cell || !particle_cell {
        return TickDispatch::Idle;
    }
    *counter = counter.wrapping_add(slices);
    match mode {
        0 => TickDispatch::Run,
        2 => TickDispatch::Restart,
        3 => TickDispatch::Quit,
        _ => TickDispatch::Idle,
    }
}
