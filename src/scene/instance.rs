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
}

struct Slot {
    operator: usize,
    state: State,
}

enum State {
    Delay(delay::State),
    Physic(Physic),
}

pub struct Physic {
    pub flags: u32,
    pub cell: Option<Rc<RefCell<DynamicCell>>>,
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
