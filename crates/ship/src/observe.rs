use std::{fmt::Write as _, future::pending, time::Duration};

use futures_util::{Stream, StreamExt};
use ship_client::Client;
use ship_core::{
    id::{Attachment, IdOf},
    model::{NodeId, Session, Tab},
    prelude::*,
    protocol::{AttachRequest, Replica, SseEvent, ViewingRecord},
};

use crate::controls;

const FIRST_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// `ship attach <session>`. Prints the tree on stdout after every applied
/// event and status lines on stderr. Stdin lines are controls (`controls.rs`);
/// a failed control is reported and the observer keeps running.
///
/// When the stream ends or fails, the observer reports the disconnection and
/// reattaches with backoff (250 ms doubling, capped at 5 s), sending the
/// remembered session ID and selection. A `404` on reattach means the session
/// was removed: it prints `session removed` and exits 0. Otherwise it exits
/// only on SIGINT, with 0.
pub async fn run(client: &Client, session: IdOf<Session>) -> Result<()> {
    let mut request = AttachRequest {
        session,
        selection: None,
    };
    let reattach = |request: AttachRequest, delay: Duration| {
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            client.attach(&request).await
        })
    };
    let mut events = Some(Box::pin(client.attach(&request).await?));
    let mut reconnect = None;
    let mut backoff = FIRST_BACKOFF;
    let mut controls = controls::lines();
    let mut stdin_open = true;
    let mut observer = Observer::default();
    loop {
        tokio::select! {
            event = next(&mut events) => {
                let reason = match event {
                    Some(Ok(event)) => {
                        if let SseEvent::Attached(attached) = &event {
                            eprintln!("attached {} to {}", attached.attachment, request.session);
                        }
                        if observer.apply(event) {
                            print!("{}", observer.render());
                        }
                        continue;
                    }
                    Some(Err(error)) => error.to_string(),
                    None => "attach stream ended".to_owned(),
                };
                events = None;
                if let Some(remembered) = observer.remembered() {
                    request = remembered;
                }
                observer.attachment = None;
                backoff = FIRST_BACKOFF;
                eprintln!("disconnected: {reason}; retrying in {backoff:?}");
                reconnect = Some(reattach(request.clone(), backoff));
            }
            result = finish(&mut reconnect) => match result {
                Ok(stream) => {
                    reconnect = None;
                    events = Some(Box::pin(stream));
                }
                Err(error) if *error.code() == ErrorCode::NotFound => {
                    eprintln!("session removed");
                    return Ok(());
                }
                Err(error) => {
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    eprintln!("disconnected: {error}; retrying in {backoff:?}");
                    reconnect = Some(reattach(request.clone(), backoff));
                }
            },
            line = controls.recv(), if stdin_open => match line {
                None => stdin_open = false,
                Some(line) if line.trim().is_empty() => {}
                Some(line) => {
                    if let Err(error) = control(client, &observer, &line).await {
                        eprintln!("ship: {error}");
                    }
                }
            },
            _ = tokio::signal::ctrl_c() => return Ok(()),
        }
    }
}

/// The next event, or never while disconnected.
async fn next<S: Stream + Unpin>(events: &mut Option<S>) -> Option<S::Item> {
    match events {
        Some(events) => events.next().await,
        None => pending().await,
    }
}

/// The pending reattach attempt's result, or never while none is pending.
async fn finish<F: Future + Unpin>(attempt: &mut Option<F>) -> F::Output {
    match attempt {
        Some(attempt) => attempt.await,
        None => pending().await,
    }
}

async fn control(client: &Client, observer: &Observer, line: &str) -> Result<()> {
    let control = controls::parse(line)?;
    let attachment = observer
        .attachment
        .ok_or_else(|| err!(Validation, "not attached"))?;
    controls::send(client, attachment, control).await
}

#[derive(Default)]
struct Observer {
    attachment: Option<IdOf<Attachment>>,
    replica: Option<Replica>,
}

impl Observer {
    /// `Attached` replaces everything. `State` applies only for the same
    /// incarnation and a higher revision. Returns whether anything changed.
    fn apply(&mut self, event: SseEvent) -> bool {
        match event {
            SseEvent::Attached(attached) => {
                self.attachment = Some(attached.attachment);
                self.replica = Some((*attached.replica).clone());
                true
            }
            SseEvent::State(replica) => match &self.replica {
                Some(current)
                    if current.incarnation == replica.incarnation
                        && current.revision < replica.revision =>
                {
                    self.replica = Some((*replica).clone());
                    true
                }
                _ => false,
            },
        }
    }

    /// This observer's record, matched by attachment ID.
    fn record(&self) -> Option<&ViewingRecord> {
        self.replica.as_ref()?.viewers.get(&self.attachment?)
    }

    /// What to send on reattach: the last record's session ID and selection.
    /// `None` before the first `Attached` event.
    fn remembered(&self) -> Option<AttachRequest> {
        let record = self.record()?;
        Some(AttachRequest {
            session: record.session,
            selection: Some(record.selection),
        })
    }

    /// Indented tree of the attached session, selection marked with `*`.
    fn render(&self) -> String {
        let (Some(replica), Some(record)) = (&self.replica, self.record()) else {
            return String::new();
        };
        let Some(session) = replica.sessions.get(&record.session) else {
            return String::new();
        };
        let mut out = String::new();
        let selection = record.selection;
        let _ = writeln!(
            out,
            "{} {:?} (revision {}){}",
            session.id,
            session.name.to_string(),
            replica.revision,
            marker(NodeId::Session(session.id), selection)
        );
        for root in session.tabs.values() {
            render_tab(&mut out, root, 1, selection);
        }
        out
    }
}

/// One tab line, its panes, then its child tabs one level deeper.
fn render_tab(out: &mut String, tab: &Tab, depth: usize, selection: NodeId) {
    let indent = "  ".repeat(depth);
    let mark = marker(NodeId::Tab(tab.id), selection);
    let _ = writeln!(out, "{indent}{} {:?}{mark}", tab.id, tab.name);
    for pane in tab.panes.values() {
        let mark = marker(NodeId::Pane(pane.id), selection);
        let _ = writeln!(out, "{indent}  {} {:?}{mark}", pane.id, pane.name);
    }
    for child in tab.tabs.values() {
        render_tab(out, child, depth + 1, selection);
    }
}

fn marker(node: NodeId, selection: NodeId) -> &'static str {
    if node == selection { " *" } else { "" }
}
