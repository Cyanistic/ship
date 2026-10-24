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
    protocol::{
        AttachRequest, EndReason, Ended, FeedMode, InputFrame, KeyInput, PasteInput, Replica,
        SseEvent,
    },
    tree,
};
use tokio::time::timeout;

use crate::Client;

/// How long `send --wait` gives the program to move off its status before the input.
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
    /// Check existence, and readiness when asked, then write without attaching.
    pub(crate) async fn send(
        &self,
        pane: IdOf<Pane>,
        input: Input,
        ready: Option<Condition>,
    ) -> Result<()> {
        let found = self.get::<Pane>(pane).await?;
        if let Some(ready) = ready {
            check(&found, &ready)?;
        }
        self.write(input.frames(pane)).await?;
        Ok(())
    }

    /// The response revision separates before and after the queued input.
    async fn write(&self, frames: Vec<InputFrame>) -> Result<u64> {
        self.input(None, stream::iter(frames)).await?;
        Ok(self.written())
    }

    /// Return at once if the snapshot already holds the condition. A pane
    /// without a root record waits for its first report.
    pub(crate) async fn wait(&self, pane: IdOf<Pane>, condition: Condition) -> Result<Pane> {
        let mut events = pin!(self.observe().await?);
        let snapshot = next(&mut events, pane).await?;
        let found = tree::pane(&snapshot.tabs, pane)?;
        if condition.matches(found)? {
            return Ok(found.clone());
        }
        watch(&mut events, pane, snapshot.revision, |found| {
            condition.matches(found)
        })
        .await
    }

    /// Refuse before writing unless ready, then, unless it was working, wait
    /// `STALL` for the program to move off its status, then for the condition.
    pub(crate) async fn send_and_wait(
        &self,
        pane: IdOf<Pane>,
        input: Input,
        ready: Condition,
        condition: Condition,
    ) -> Result<Pane> {
        let mut events = pin!(self.observe().await.context("nothing was sent")?);
        let snapshot = next(&mut events, pane).await.context("nothing was sent")?;
        let before = tree::pane(&snapshot.tabs, pane)
            .and_then(|found| check(found, &ready).map(|()| found.root_status().cloned()))
            .context("nothing was sent")?;
        let after = self
            .write(input.frames(pane))
            .await
            .context("the input may have been sent")?;
        let handled = async {
            if before
                .as_ref()
                .is_none_or(|record| record.state != ReportedState::Working)
            {
                let moved = timeout(
                    STALL,
                    watch(&mut events, pane, after, |found| {
                        Ok(moved_from(found, before.as_ref()))
                    }),
                )
                .await
                .map_err(|_| {
                    err!(
                        Conflict,
                        "pane {} stalled: its program status did not change within 5 seconds",
                        pane
                    )
                })??;
                if condition.matches(&moved)? {
                    return Ok(moved);
                }
            }
            watch(&mut events, pane, after, |found| condition.matches(found)).await
        };
        handled.await.context("the input was sent")
    }

    /// An every-revision observer: no selection or area, so it moves and sizes nothing.
    async fn observe(&self) -> Result<impl Stream<Item = Result<SseEvent>> + use<>> {
        self.attach(&AttachRequest {
            selection: None,
            area: None,
            feed: FeedMode::Every,
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
            Some(Ok(SseEvent::Ended(Ended {
                reason: EndReason::Lost,
            }))) => {
                return Err(err!(
                    Unavailable,
                    "fell behind the server while waiting on pane {}",
                    pane
                ));
            }
            Some(Ok(SseEvent::Ended(Ended {
                reason: EndReason::ServerShutdown,
            }))) => {
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

/// Refuse unless the root reports a ready state; exit fails with its status.
fn check(pane: &Pane, ready: &Condition) -> Result<()> {
    if ready.matches(pane)? {
        return Ok(());
    }
    Err(match pane.root_status() {
        None => err!(Conflict, "pane {} has no program status", pane.id),
        Some(record) if record.state == ReportedState::Blocked => {
            let kind: &str = record.kind.map_or("unknown", Into::into);
            err!(Conflict, "pane {} is blocked on {}", pane.id, kind)
        }
        Some(record) => {
            let state: &str = record.state.into();
            err!(Conflict, "pane {} is {}", pane.id, state)
        }
    })
}

/// The program left `before`: it exited or reports another state or blocked
/// kind. A new title or message alone doesn't count.
fn moved_from(pane: &Pane, before: Option<&StatusRecord>) -> bool {
    matches!(pane.status, PaneStatus::Exited(_))
        || pane.root_status().map(|record| (record.state, record.kind))
            != before.map(|record| (record.state, record.kind))
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

    /// `send --wait` without `--if`: anything reported but blocked.
    pub(crate) fn unblocked() -> Self {
        Self::Any(vec![
            WaitState::Idle,
            WaitState::Working,
            WaitState::Done,
            WaitState::Error,
        ])
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
