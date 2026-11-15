//! Persistent actor identities plus the original per-frame registration list.
use super::monster_actor::Actor;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub type Handle = Rc<RefCell<Actor>>;
#[derive(Default)]
pub struct Registry {
    next: usize,
    actors: BTreeMap<usize, Handle>,
    active: Vec<usize>,
}
impl Registry {
    /// Allocate instance-owned storage; the scene operator initializes it before
    /// registration. Identities are never reused, so removed actors stay invalid.
    pub fn allocate(&mut self) -> Result<Handle, &'static str> {
        let id = self.next;
        self.next = id.checked_add(1).ok_or("monster identity exhausted")?;
        let actor = Rc::new(RefCell::new(Actor::uninitialized(id)));
        self.actors.insert(id, Rc::clone(&actor));
        Ok(actor)
    }
    pub fn get(&self, id: usize) -> Option<Handle> {
        self.actors.get(&id).map(Rc::clone)
    }
    pub fn begin_frame(&mut self) {
        self.active.clear();
    }
    pub fn register(&mut self, id: usize) -> Result<(), &'static str> {
        if !self.actors.contains_key(&id) {
            return Err("monster registration identity missing");
        }
        self.active.push(id);
        Ok(())
    }
    pub fn active_ids(&self) -> &[usize] {
        &self.active
    }
    pub fn active(&self) -> impl Iterator<Item = Handle> + '_ {
        self.active.iter().map(|id| Rc::clone(&self.actors[id]))
    }
    pub fn len(&self) -> usize {
        self.active.len()
    }
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }
    pub fn remove(&mut self, id: usize) -> Option<Handle> {
        self.active.retain(|&entry| entry != id);
        self.actors.remove(&id)
    }
}
