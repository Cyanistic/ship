//! CLI input and waits. Input needs no attachment; waits observe published
//! replicas without a selection or area and detach when their stream drops.

use std::{
    io::{self, Read},
    pin::pin,
    sync::Arc,
    time::Duration,
};

use crokey::KeyCombination;
use crossterm::event::KeyEvent;
use futures_util::{Stream, StreamExt, stream};
use ship_core::{
    command::pane::WaitState,
    id::IdOf,
    model::{ExitStatus, Pane, PaneStatus, ReportedState, StatusRecord},
    prelude::*,
    protocol::{AttachRequest, InputFrame, KeyInput, PasteInput, Replica, SseEvent},
    tree,
};
use tokio::time::timeout;

use crate::Client;

/// How long `send --wait` gives the program to report `working` or `blocked`.
const STALL: Duration = Duration::from_secs(5);

/// One paste followed by keys, all parsed before writing.
pub(crate) struct Input {
    text: Option<String>,
    keys: Vec<KeyEvent>,
}

impl Input {
    pub(crate) fn new(text: Option<String>, keys: Vec<KeyCombination>) -> Result<Self> {
        let text = match text.as_deref() {
            Some("-") => {
                let mut text = String::new();
                io::stdin()
                    .read_to_string(&mut text)
                    .map_err(|error| err!(Io, "cannot read input from stdin", @external: error))?;
                Some(text)
            }
            _ => text,
        };
        Ok(Self {
            text,
            keys: keys.into_iter().map(Into::into).collect(),
        })
    }

    fn frames(self, pane: IdOf<Pane>) -> Vec<InputFrame> {
        self.text
            .into_iter()
            .map(|text| InputFrame::Paste(PasteInput { pane, text }))
            .chain(
                self.keys
                    .into_iter()
                    .map(|key| InputFrame::Key(KeyInput { pane, key })),
            )
            .collect()
    }
}

impl Client {
    /// Check existence and write without attaching.
    pub(crate) async fn send(&self, pane: IdOf<Pane>, input: Input) -> Result<()> {
        self.get::<Pane>(pane).await?;
        self.write(input.frames(pane)).await?;
        Ok(())
    }

    /// The response revision separates before and after the queued input.
    async fn write(&self, frames: Vec<InputFrame>) -> Result<u64> {
        self.input(None, stream::iter(frames)).await?;
        Ok(self.written())
    }

    /// Return at once if the snapshot already holds the condition.
    pub(crate) async fn wait(&self, pane: IdOf<Pane>, condition: Condition) -> Result<Pane> {
        let mut events = pin!(self.observe().await?);
        let snapshot = next(&mut events, pane).await?;
        let found = tree::pane(&snapshot.tabs, pane)?;
        if found.status == PaneStatus::Running
            && found.root_status().is_none()
            && !condition.takes_exit()
        {
            return Err(err!(Conflict, "pane {} has no program status", pane));
        }
        if condition.matches(found)? {
            return Ok(found.clone());
        }
        watch(&mut events, pane, snapshot.revision, |found| {
            condition.matches(found)
        })
        .await
    }

    /// Refuse before writing, then wait for the program to pick up the input
    /// within `STALL` unless it was already working, then for the condition.
    pub(crate) async fn send_and_wait(
        &self,
        pane: IdOf<Pane>,
        input: Input,
        condition: Condition,
    ) -> Result<Pane> {
        let mut events = pin!(self.observe().await.context("nothing was sent")?);
        let snapshot = next(&mut events, pane).await.context("nothing was sent")?;
        let working = tree::pane(&snapshot.tabs, pane)
            .and_then(accepts_input)
            .context("nothing was sent")?;
        let after = self
            .write(input.frames(pane))
            .await
            .context("the input may have been sent")?;
        let handled = async {
            if !working {
                let picked = timeout(
                    STALL,
                    watch(&mut events, pane, after, |found| Ok(picked_up(found))),
                )
                .await
                .map_err(|_| {
                    err!(
                        Conflict,
                        "pane {} stalled: no working or blocked within 5 seconds",
                        pane
                    )
                })??;
                if condition.matches(&picked)? {
                    return Ok(picked);
                }
            }
            watch(&mut events, pane, after, |found| condition.matches(found)).await
        };
        handled.await.context("the input was sent")
    }

    /// An observer: no selection and no area, so it neither moves nor sizes anything.
    async fn observe(&self) -> Result<impl Stream<Item = Result<SseEvent>> + use<>> {
        self.attach(&AttachRequest {
            selection: None,
            area: None,
        })
        .await
    }
}

/// The next published replica. An observer gets no screens or clipboard.
async fn next(
    events: &mut (impl Stream<Item = Result<SseEvent>> + Unpin),
    pane: IdOf<Pane>,
) -> Result<Arc<Replica>> {
    loop {
        match events.next().await {
            Some(Ok(SseEvent::Attached(attached))) => return Ok(attached.replica),
            Some(Ok(SseEvent::State(replica))) => return Ok(replica),
            Some(Ok(SseEvent::Screen(_) | SseEvent::Clipboard(_))) => {}
            Some(Ok(SseEvent::Ended(_))) => {
                return Err(err!(
                    Unavailable,
                    "the server stopped while waiting on pane {}",
                    pane
                ));
            }
            Some(Err(error)) => {
                return Err(
                    error.context(format!("lost the connection while waiting on pane {pane}"))
                );
            }
            None => {
                return Err(err!(
                    Network,
                    "lost the connection while waiting on pane {}",
                    pane
                ));
            }
        }
    }
}

/// The pane from the first replica newer than `after` that `done` accepts.
async fn watch(
    events: &mut (impl Stream<Item = Result<SseEvent>> + Unpin),
    pane: IdOf<Pane>,
    after: u64,
    mut done: impl FnMut(&Pane) -> Result<bool>,
) -> Result<Pane> {
    loop {
        let replica = next(events, pane).await?;
        if replica.revision <= after {
            continue;
        }
        let found =
            tree::pane(&replica.tabs, pane).map_err(|_| err!(NotFound, "pane {} closed", pane))?;
        if done(found)? {
            return Ok(found.clone());
        }
    }
}

/// Whether the root is already working; exited, unreported and blocked panes
/// refuse input.
fn accepts_input(pane: &Pane) -> Result<bool> {
    if let PaneStatus::Exited(status) = &pane.status {
        return Err(exited(pane.id, status));
    }
    let Some(record) = pane.root_status() else {
        return Err(err!(Conflict, "pane {} has no program status", pane.id));
    };
    if record.state == ReportedState::Blocked {
        let kind: &str = record.kind.map_or("unknown", Into::into);
        return Err(err!(Conflict, "pane {} is blocked on {}", pane.id, kind));
    }
    Ok(record.state == ReportedState::Working)
}

/// Seeing the program take the input: working, blocked, or gone.
fn picked_up(pane: &Pane) -> bool {
    matches!(pane.status, PaneStatus::Exited(_))
        || pane.root_status().is_some_and(|record| {
            matches!(
                record.state,
                ReportedState::Working | ReportedState::Blocked
            )
        })
}

/// The default differs from listing every non-working state: it never takes exit.
pub(crate) enum Condition {
    NotWorking,
    Any(Vec<WaitState>),
}

impl Condition {
    pub(crate) fn new(states: Vec<WaitState>) -> Self {
        if states.is_empty() {
            Self::NotWorking
        } else {
            Self::Any(states)
        }
    }

    /// Exit matches only when asked for and otherwise fails with its status.
    /// A missing root record matches nothing.
    fn matches(&self, pane: &Pane) -> Result<bool> {
        if let PaneStatus::Exited(status) = &pane.status {
            return if self.takes_exit() {
                Ok(true)
            } else {
                Err(exited(pane.id, status))
            };
        }
        Ok(pane.root_status().is_some_and(|record| self.holds(record)))
    }

    fn holds(&self, record: &StatusRecord) -> bool {
        match self {
            Self::NotWorking => record.state != ReportedState::Working,
            Self::Any(states) => states.contains(&record.state.into()),
        }
    }

    fn takes_exit(&self) -> bool {
        matches!(self, Self::Any(states) if states.contains(&WaitState::Exited))
    }
}

fn exited(pane: IdOf<Pane>, status: &ExitStatus) -> AppError {
    match &status.signal {
        Some(signal) => err!(
            Conflict,
            "pane {} program exited: killed by {}",
            pane,
            signal
        ),
        None => err!(
            Conflict,
            "pane {} program exited with code {}",
            pane,
            status.code
        ),
    }
}
