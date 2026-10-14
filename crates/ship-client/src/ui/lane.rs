//! Everything the user does, in order, one turn at a time.

use std::collections::VecDeque;

use crossterm::event::KeyEvent;
use ship_core::{
    id::IdOf,
    layout::Axis,
    model::{NodeId, Pane, Tab},
    prelude::*,
    protocol::{InputFrame, MoveTab},
};

use super::{Request, finish, observer::Observer};
use crate::keymap::Action;

/// A shared border, named by its left or upper pane.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Edge {
    pub pane: IdOf<Pane>,
    pub axis: Axis,
}

#[derive(Clone, Copy)]
pub(super) struct EdgeAt {
    pub edge: Edge,
    pub to: u16,
}

/// A dropped tab keeps its explicit destination until its turn.
#[derive(Clone, Copy)]
pub(super) struct TabMove {
    pub tab: IdOf<Tab>,
    pub to: MoveTab,
}

/// Selection-dependent targets are resolved only when this item runs.
pub(super) enum Item {
    Action(Action),
    Key(KeyEvent),
    Paste(String),
    /// Clicks keep the target hit at press.
    Select(NodeId),
    Fold(IdOf<Tab>),
    Mouse(InputFrame),
    Resize(EdgeAt),
    Move(TabMove),
}

pub(super) enum Turn<'a> {
    Done,
    Wait(Request<'a>),
    /// Run these first, in order, including any item that needs retrying.
    First(Vec<Item>),
    Detach,
}

#[derive(Default)]
pub(super) struct Lane<'a> {
    items: VecDeque<Item>,
    running: Option<Request<'a>>,
}

impl<'a> Lane<'a> {
    pub fn push(&mut self, item: Item) {
        if let Item::Resize(position) = &item
            && let Some(Item::Resize(waiting)) = self
                .items
                .iter_mut()
                .find(|item| matches!(item, Item::Resize(waiting) if waiting.edge == position.edge))
        {
            *waiting = *position;
            return;
        }
        self.items.push_back(item);
    }

    pub fn first(&mut self, items: Vec<Item>) {
        for item in items.into_iter().rev() {
            self.items.push_front(item);
        }
    }

    pub fn busy(&self) -> bool {
        self.running.is_some() || !self.items.is_empty()
    }

    pub fn next(&mut self, observer: &Observer, written: u64) -> Option<Item> {
        if observer.connected && self.running.is_none() && observer.caught_up(written) {
            self.items.pop_front()
        } else {
            None
        }
    }

    pub fn wait(&mut self, request: Request<'a>) {
        self.running = Some(request);
    }

    pub async fn finished(&mut self) -> Result<()> {
        let result = finish(&mut self.running).await;
        self.running = None;
        result
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.running = None;
    }
}
