//! Sequential KEnvironment::GetInst storage and ExecWithNewMem scopes.
use super::{Error, delay};
use crate::{
    collision::{Cell, DynamicCell},
    mesh::{Matrix, identity},
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    #[default]
    Default,
    Scene(usize),
    Event(super::event::Id),
}

struct Slot {
    operator: usize,
    state: State,
}

enum State {
    Delay(delay::State),
    Sample(super::sample::State),
    Emitter(super::particles::Emitter),
    Scatter(super::scatter::State),
    Walk(Box<super::walk::State>),
    Physic(Physic),
    Splash(bool),
    Trigger(super::trigger::State),
    Monster(Option<Monster>),
}

#[derive(Clone, Copy, Debug)]
pub struct Monster {
    pub released: bool,
    pub actor: usize,
    pub live: super::event::Id,
    pub death: super::event::Id,
}

pub struct Physic {
    pub flags: u32,
    pub cell: Option<Rc<RefCell<DynamicCell>>>,
    pub point: Option<Rc<RefCell<crate::game::beta_monster::projectile::Point>>>,
}

#[derive(Clone, Copy, Default)]
pub struct Cursor {
    scope: Scope,
    next: usize,
    allocating: bool,
}

#[derive(Default)]
pub struct Memory {
    lists: BTreeMap<Scope, Vec<Slot>>,
    cursor: Cursor,
}

impl Memory {
    pub fn contains(&self, scope: Scope) -> bool {
        self.lists.get(&scope).is_some_and(|list| !list.is_empty())
    }
    pub fn clear(&mut self, scope: Scope) -> Vec<Monster> {
        self.lists
            .remove(&scope)
            .into_iter()
            .flatten()
            .filter_map(|slot| match slot.state {
                State::Monster(monster) => monster,
                _ => None,
            })
            .collect()
    }
    /// KEvent::Init loses the old instance chain without DeleteChain.
    pub fn forget(&mut self, scope: Scope) {
        self.lists.remove(&scope);
    }
    pub fn set_last_monster(&mut self, operator: usize, value: Monster) -> Result<(), Error> {
        let fail = || Error {
            operator,
            reason: "monster instance slot missing",
        };
        let index = self.cursor.next.checked_sub(1).ok_or_else(fail)?;
        let slot = self
            .lists
            .get_mut(&self.cursor.scope)
            .and_then(|list| list.get_mut(index))
            .ok_or_else(fail)?;
        if slot.operator != operator {
            return Err(fail());
        }
        match &mut slot.state {
            State::Monster(target) => {
                *target = Some(value);
                Ok(())
            }
            _ => Err(fail()),
        }
    }
    pub fn monster(&mut self, operator: usize) -> Result<&mut Option<Monster>, Error> {
        match self.next(operator, || State::Monster(None))? {
            State::Monster(monster) => Ok(monster),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    pub fn walk(&mut self, operator: usize) -> Result<&mut super::walk::State, Error> {
        match self.next(operator, || State::Walk(Box::default()))? {
            State::Walk(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    pub fn scatter(&mut self, operator: usize) -> Result<&mut super::scatter::State, Error> {
        match self.next(
            operator,
            || State::Scatter(super::scatter::State::default()),
        )? {
            State::Scatter(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    pub fn emitter(&mut self, operator: usize) -> Result<&mut super::particles::Emitter, Error> {
        match self.next(operator, || {
            State::Emitter(super::particles::Emitter::default())
        })? {
            State::Emitter(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    pub fn sample(&mut self, operator: usize) -> Result<&mut super::sample::State, Error> {
        match self.next(operator, || State::Sample(super::sample::State::default()))? {
            State::Sample(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    pub fn trigger(&mut self, operator: usize) -> Result<&mut super::trigger::State, Error> {
        match self.next(
            operator,
            || State::Trigger(super::trigger::State::default()),
        )? {
            State::Trigger(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    pub fn splash(&mut self, operator: usize) -> Result<&mut bool, Error> {
        match self.next(operator, || State::Splash(false))? {
            State::Splash(previous) => Ok(previous),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
    /// Save the caller's list cursor and start at the selected list head.
    pub fn enter(&mut self, scope: Scope) -> Cursor {
        let saved = self.cursor;
        self.cursor = Cursor {
            scope,
            next: 0,
            allocating: self.lists.get(&scope).is_none_or(Vec::is_empty),
        };
        saved
    }

    pub fn restore(&mut self, saved: Cursor) {
        self.cursor = saved;
    }

    fn next(
        &mut self,
        operator: usize,
        create: impl FnOnce() -> State,
    ) -> Result<&mut State, Error> {
        let list = self.lists.entry(self.cursor.scope).or_default();
        let fail = |reason| Error { operator, reason };
        let index = self.cursor.next;
        if index == list.len() && self.cursor.allocating {
            list.push(Slot {
                operator,
                state: create(),
            });
        }
        let slot = list
            .get_mut(index)
            .ok_or_else(|| fail("instance list exhausted during replay"))?;
        if slot.operator != operator {
            return Err(fail("instance operator differs during replay"));
        }
        self.cursor.next += 1;
        Ok(&mut slot.state)
    }

    pub fn delay(&mut self, operator: usize) -> Result<&mut delay::State, Error> {
        match self.next(operator, || State::Delay(delay::State::default()))? {
            State::Delay(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }

    pub fn physic(
        &mut self,
        operator: usize,
        flags: u32,
        scale: [f32; 3],
        matrix: Matrix,
    ) -> Result<&mut Physic, Error> {
        match self.next(operator, || {
            State::Physic(Physic {
                flags,
                point: None,
                cell: (flags & 3 == 2).then(|| {
                    Rc::new(RefCell::new(DynamicCell::new(
                        Cell::new(identity(), scale, 1),
                        matrix,
                    )))
                }),
            })
        })? {
            State::Physic(state) => Ok(state),
            _ => Err(Error {
                operator,
                reason: "instance memory type differs during replay",
            }),
        }
    }
}
