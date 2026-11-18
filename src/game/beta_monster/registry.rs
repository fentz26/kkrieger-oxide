//! Persistent released actors and ordered per-frame registration.
use super::actor::Actor;
use crate::mesh::Matrix;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub type Handle = Rc<RefCell<Actor>>;
#[derive(Default)]
pub struct Registry {
    next: u32,
    actors: BTreeMap<u32, Handle>,
    active: Vec<u32>,
}
impl Registry {
    pub fn spawn<F>(
        &mut self,
        words: &[u32],
        parent: Matrix,
        attack_operator: u32,
        find_cell: F,
    ) -> Result<u32, &'static str>
    where
        F: FnMut([f32; 4]) -> Option<usize>,
    {
        let token = self
            .next
            .checked_add(1)
            .ok_or("released actor identity exhausted")?;
        let actor = Actor::spawn(words, parent, attack_operator, token, find_cell)?;
        self.actors.insert(token, Rc::new(RefCell::new(actor)));
        self.next = token;
        Ok(token)
    }
    pub fn get(&self, token: u32) -> Option<Handle> {
        self.actors.get(&token).map(Rc::clone)
    }
    pub fn begin_frame(&mut self) {
        self.active.clear();
    }
    pub fn register(&mut self, token: u32) -> Result<(), &'static str> {
        if !self.actors.contains_key(&token) {
            return Err("released actor registration missing");
        }
        self.active.push(token);
        Ok(())
    }
    pub fn active_ids(&self) -> &[u32] {
        &self.active
    }
    pub fn active(&self) -> impl Iterator<Item = Handle> + '_ {
        self.active
            .iter()
            .map(|token| Rc::clone(&self.actors[token]))
    }
    /// Visit persistent storage in registration order, releasing each borrow
    /// before the next entry so repeated registrations remain valid.
    pub fn visit_active<F>(&self, visit: F) -> Result<(), &'static str>
    where
        F: FnMut(u32, &mut Actor) -> Result<(), &'static str>,
    {
        self.visit_tokens(&self.active, visit)
    }
    pub fn visit_tokens<F>(&self, tokens: &[u32], mut visit: F) -> Result<(), &'static str>
    where
        F: FnMut(u32, &mut Actor) -> Result<(), &'static str>,
    {
        for &token in tokens {
            let handle = self
                .actors
                .get(&token)
                .ok_or("released actor identity missing")?;
            let mut actor = handle
                .try_borrow_mut()
                .map_err(|_| "released actor already borrowed")?;
            visit(token, &mut actor)?;
        }
        Ok(())
    }

    pub fn remove(&mut self, token: u32) -> Option<Handle> {
        self.active.retain(|&id| id != token);
        self.actors.remove(&token)
    }
}
