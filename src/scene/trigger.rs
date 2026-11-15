//! Original Misc_Trigger state and dynamic-event creation.
use super::event::Event;
use crate::mesh::Matrix;
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub active: bool,
    pub time: i32,
    initialized: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub operator: usize,
    pub trigger: f32,
    pub threshold: f32,
    pub delay: f32,
    pub duration: f32,
    pub flags: u32,
    pub velocity: f32,
    pub modulation: f32,
    pub select: i32,
    pub scale: [f32; 3],
    pub rotate: [f32; 3],
    pub translate: [f32; 3],
}
impl State {
    pub fn advance(&mut self, beat: i32, p: Parameters, parent: Matrix) -> Option<Event> {
        if !self.initialized {
            self.initialized = true;
            self.active = true;
            self.time = beat;
        }
        let mut doit = p.trigger > p.threshold;
        if p.flags & 1 != 0 {
            doit = !doit;
        }
        if !doit {
            self.time = beat;
        }
        if p.flags & 2 != 0 && self.active && beat >= self.time {
            self.active = false;
        }
        let event = if doit && !self.active {
            let mut translate = [p.translate[0], p.translate[1], p.translate[2], 0.];
            if p.flags & 4 != 0 {
                translate = parent[3];
                translate[3] = 0.;
            }
            let event = Event {
                operator: Some(p.operator),
                start: self.time,
                end: (self.time as f32 + p.duration * 65536.) as i32,
                velocity: p.velocity,
                modulation: p.modulation,
                select: p.select,
                scale: [p.scale[0], p.scale[1], p.scale[2], 0.],
                rotate: [p.rotate[0], p.rotate[1], p.rotate[2], 0.],
                translate,
                ..Event::default()
            };
            self.active = true;
            self.time = (self.time as f32 + p.delay * 65536.) as i32;
            Some(event)
        } else {
            None
        };
        if !doit {
            self.active = false;
        }
        event
    }
}
