//! Original KEvent values and KEnvironment event scheduling.
use crate::{
    kx::Document,
    mesh::{Matrix, identity},
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub type Handle = Rc<RefCell<Event>>;
pub type Id = u64;
#[derive(Clone, Debug)]
pub struct Event {
    pub matrix: Matrix,
    pub operator: Option<usize>,
    pub start: i32,
    pub end: i32,
    pub shot_time: i32,
    pub velocity: f32,
    pub modulation: f32,
    pub select: i32,
    pub scale: [f32; 4],
    pub rotate: [f32; 4],
    pub translate: [f32; 4],
    pub color: u32,
    pub spline: Option<usize>,
    pub interval: [f32; 2],
    pub monster: Option<usize>,
    pub cull_distance: i32,
}
impl Default for Event {
    fn default() -> Self {
        Self {
            matrix: identity(),
            operator: None,
            start: 0,
            end: 0,
            shot_time: 0,
            velocity: 0.,
            modulation: 0.,
            select: 0,
            scale: [0.; 4],
            rotate: [0.; 4],
            translate: [0.; 4],
            color: 0,
            spline: None,
            interval: [0., 1.],
            monster: None,
            cull_distance: 0,
        }
    }
}
impl Event {
    pub fn stop(&mut self) {
        self.start = 0;
        self.end = 1;
    }
    pub fn variables(&self, beat: i32) -> [[f32; 4]; 8] {
        let time = if self.start < self.end {
            beat.wrapping_sub(self.start) as f32 / self.end.wrapping_sub(self.start) as f32
        } else {
            0.
        };
        let time = self.interval[0] + time.clamp(0., 1.) * (self.interval[1] - self.interval[0]);
        [
            [time; 4],
            [self.velocity; 4],
            [self.modulation; 4],
            [self.select as f32; 4],
            self.scale,
            self.rotate,
            self.translate,
            [
                ((self.color >> 16) & 255) as f32 / 255.,
                ((self.color >> 8) & 255) as f32 / 255.,
                (self.color & 255) as f32 / 255.,
                (self.color >> 24) as f32 / 255.,
            ],
        ]
    }
}
#[derive(Default)]
pub struct Scheduler {
    pub beat: i32,
    next: Id,
    pub events: BTreeMap<Id, Handle>,
    pub document: Vec<Id>,
    pub dynamic: Vec<Id>,
    active: BTreeMap<usize, Vec<Id>>,
}
impl Scheduler {
    pub fn from_document(doc: &Document<'_>) -> Self {
        let mut scheduler = Self::default();
        for source in &doc.events {
            let event = Event {
                operator: Some(source.operator),
                start: source.start,
                end: source.end,
                velocity: source.velocity,
                modulation: source.modulation,
                select: i32::from(source.select),
                scale: [source.scale[0], source.scale[1], source.scale[2], 0.],
                rotate: [source.rotate[0], source.rotate[1], source.rotate[2], 0.],
                translate: [
                    source.translate[0],
                    source.translate[1],
                    source.translate[2],
                    0.,
                ],
                color: source.color,
                spline: source.spline,
                interval: source.interval,
                ..Event::default()
            };
            let id = scheduler.insert(event, false);
            scheduler.document.push(id);
        }
        scheduler
    }
    pub fn insert(&mut self, event: Event, dynamic: bool) -> Id {
        self.insert_shared(Rc::new(RefCell::new(event)), dynamic)
    }
    pub fn insert_shared(&mut self, event: Handle, dynamic: bool) -> Id {
        let id = self.next;
        self.next = self.next.checked_add(1).expect("event identity exhausted");
        self.events.insert(id, event);
        if dynamic {
            self.dynamic.push(id);
        }
        id
    }
    pub fn reset_view(&mut self) -> Vec<Id> {
        self.beat = 0;
        self.active.clear();
        let removed = std::mem::take(&mut self.dynamic);
        for id in &removed {
            self.events.remove(id);
        }
        removed
    }
    /// Register active dynamic events first, as InitFrame does. Returned IDs need
    /// their instance lists reset when ClearInstanceMem is set.
    pub fn begin(&mut self, beat: i32, clear_instances: bool) -> Vec<Id> {
        self.beat = beat;
        self.active.clear();
        let ids = self.dynamic.clone();
        ids.into_iter()
            .filter(|&id| self.add(id) && clear_instances)
            .collect()
    }
    pub fn add_document(&mut self, clear_instances: bool) -> Vec<Id> {
        let ids = self.document.clone();
        ids.into_iter()
            .filter(|&id| self.add(id) && clear_instances)
            .collect()
    }
    pub fn add(&mut self, id: Id) -> bool {
        let Some(event) = self.events.get(&id) else {
            return false;
        };
        let event = event.borrow();
        if (event.start == event.end || (self.beat >= event.start && self.beat < event.end))
            && let Some(operator) = event.operator
        {
            self.active.entry(operator).or_default().insert(0, id);
            return true;
        }
        false
    }
    pub fn active(&self, operator: usize) -> &[Id] {
        self.active.get(&operator).map_or(&[], Vec::as_slice)
    }
    /// ExitFrame swaps expired dynamic events with the last list entry. Preserve
    /// that order because it controls next frame's event traversal.
    pub fn finish(&mut self) -> Vec<Id> {
        self.active.clear();
        let mut expired = Vec::new();
        let mut i = 0;
        while i < self.dynamic.len() {
            let id = self.dynamic[i];
            let expired_event = {
                let event = self.events[&id].borrow();
                event.end != event.start && self.beat >= event.end
            };
            if expired_event {
                self.dynamic.swap_remove(i);
                self.events.remove(&id);
                expired.push(id);
            } else {
                i += 1;
            }
        }
        expired
    }
}
