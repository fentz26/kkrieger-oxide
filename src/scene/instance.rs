//! Sequential KEnvironment::GetInst storage and ExecWithNewMem scopes.
use super::{Error, delay};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    #[default]
    Default,
    Scene(usize),
}

struct Slot {
    operator: usize,
    delay: delay::State,
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

    pub fn delay(&mut self, operator: usize) -> Result<&mut delay::State, Error> {
        let list = self.lists.entry(self.cursor.scope).or_default();
        let fail = |reason| Error { operator, reason };
        let index = self.cursor.next;
        if index == list.len() && self.cursor.allocating {
            list.push(Slot {
                operator,
                delay: delay::State::default(),
            });
        }
        let slot = list
            .get_mut(index)
            .ok_or_else(|| fail("instance list exhausted during replay"))?;
        if slot.operator != operator {
            return Err(fail("instance operator differs during replay"));
        }
        self.cursor.next += 1;
        Ok(&mut slot.delay)
    }
}
