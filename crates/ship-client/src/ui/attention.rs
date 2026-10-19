//! When a change in a root record becomes a notification or an unseen mark.

use std::{
    collections::HashMap,
    mem,
    time::{Duration, Instant},
};

use ship_core::{
    id::IdOf,
    model::{Pane, ReportedState},
    tree,
};

use super::{agents, observer::Observer};
use crate::keymap::Notice;

/// A notification waiting out the delay.
struct Pending {
    notice: Notice,
    due: Instant,
}

/// A fired notification, carrying what the popup and sound need.
pub(super) struct Fired {
    pub notice: Notice,
    /// The record's `app`, which picks the sound.
    pub app: Option<String>,
    /// The app, else the pane's label.
    pub name: String,
    pub path: String,
    pub message: Option<String>,
}

impl Fired {
    /// `<app> needs you · <path>` or `<app> finished · <path>`, then
    /// `: <msg>` when the record has one.
    pub fn popup(&self) -> String {
        let verb = match self.notice {
            Notice::Attention => "needs you",
            Notice::Done => "finished",
        };
        let mut line = format!("{} {verb} · {}", self.name, self.path);
        if let Some(message) = &self.message {
            line.push_str(": ");
            line.push_str(message);
        }
        line
    }
}

pub(super) struct Attention {
    /// Panes with a root record in the last replica seen.
    panes: HashMap<IdOf<Pane>, Watched>,
    /// Whether the client's window has focus. Starts true, so a terminal
    /// that never reports focus counts as focused.
    focused: bool,
}

/// What the notifier knows about one pane. A pane can have a pending notice
/// and an unseen mark at once: it finished unseen, then finished again.
struct Watched {
    /// Its root state in the last replica seen.
    state: ReportedState,
    pending: Option<Pending>,
    unseen: bool,
}

/// No panes, focused.
impl Default for Attention {
    fn default() -> Self {
        Self {
            panes: HashMap::new(),
            focused: true,
        }
    }
}

impl Attention {
    /// Forget everything and take the replica as the baseline, so nothing
    /// that happened before the attach notifies.
    pub fn attached(&mut self, observer: &Observer) {
        self.panes = roots(observer)
            .map(|(pane, state)| {
                let watched = Watched {
                    state,
                    pending: None,
                    unseen: false,
                };
                (pane, watched)
            })
            .collect();
    }

    /// Turn root-state changes into notices due after `delay`, mark a
    /// finished pane unseen unless it's in view while focused, and forget
    /// panes that lost their root record.
    pub fn observe(&mut self, observer: &Observer, now: Instant, delay: Duration) {
        let mut previous = mem::take(&mut self.panes);
        let focused = self.focused;
        self.panes = roots(observer)
            .map(|(pane, state)| {
                let watched = match previous.remove(&pane) {
                    Some(watched) if watched.state == state => watched,
                    // Changed, or new to the replica and so changed from nothing.
                    known => {
                        let from = known.as_ref().map(|watched| watched.state);
                        let mut watched = known.unwrap_or(Watched {
                            state,
                            pending: None,
                            unseen: false,
                        });
                        watched.state = state;
                        if let Some(notice) = notice(from, state) {
                            if notice == Notice::Done && !(focused && observer.in_view(pane)) {
                                watched.unseen = true;
                            }
                            watched.pending = Some(Pending {
                                notice,
                                due: now + delay,
                            });
                        }
                        watched
                    }
                };
                (pane, watched)
            })
            .collect();
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Clear unseen marks for panes in view while focused.
    pub fn viewed(&mut self, observer: &Observer) {
        if self.focused {
            for (pane, watched) in &mut self.panes {
                if observer.in_view(*pane) {
                    watched.unseen = false;
                }
            }
        }
    }

    pub fn due(&self) -> Option<Instant> {
        self.panes
            .values()
            .filter_map(|watched| Some(watched.pending.as_ref()?.due))
            .min()
    }

    /// Notices whose state held, minus those in view while focused.
    pub fn fire(&mut self, observer: &Observer, now: Instant) -> Vec<Fired> {
        let mut firing = Vec::new();
        for (pane, watched) in &mut self.panes {
            let Some(pending) = watched.pending.take_if(|pending| pending.due <= now) else {
                continue;
            };
            if holds(pending.notice, watched.state) && !(self.focused && observer.in_view(*pane)) {
                firing.push((*pane, pending.notice));
            }
        }
        let Some(replica) = &observer.replica else {
            return Vec::new();
        };
        let agents = agents::list(&replica.tabs, self);
        firing
            .into_iter()
            .filter_map(|(pane, notice)| {
                let agent = agents.iter().find(|agent| agent.pane.id == pane)?;
                Some(Fired {
                    notice,
                    app: agent.root.app.clone(),
                    name: agent.name().to_owned(),
                    path: agent.tab(),
                    message: agent.root.msg.clone(),
                })
            })
            .collect()
    }

    pub fn unseen(&self, pane: IdOf<Pane>) -> bool {
        self.panes.get(&pane).is_some_and(|watched| watched.unseen)
    }
}

/// Becoming blocked or error asks for attention; leaving working for idle or
/// done is a finish. `previous` is `None` for a pane new to the replica.
fn notice(previous: Option<ReportedState>, state: ReportedState) -> Option<Notice> {
    match (previous, state) {
        (_, ReportedState::Blocked | ReportedState::Error) => Some(Notice::Attention),
        (Some(ReportedState::Working), ReportedState::Idle | ReportedState::Done) => {
            Some(Notice::Done)
        }
        _ => None,
    }
}

/// Whether `state` still says what `notice` announced, once the delay ends.
fn holds(notice: Notice, state: ReportedState) -> bool {
    match notice {
        Notice::Attention => matches!(state, ReportedState::Blocked | ReportedState::Error),
        Notice::Done => matches!(state, ReportedState::Idle | ReportedState::Done),
    }
}

/// Each pane with a root record, and its root state.
fn roots(observer: &Observer) -> impl Iterator<Item = (IdOf<Pane>, ReportedState)> + '_ {
    observer.replica.iter().flat_map(|replica| {
        tree::pane_ids(&replica.tabs).filter_map(|id| {
            let pane = tree::pane(&replica.tabs, id).ok()?;
            Some((id, pane.root_status()?.state))
        })
    })
}
