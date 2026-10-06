use std::fmt::Write as _;

use futures_util::StreamExt;
use ship_client::Client;
use ship_core::{
    id::{Attachment, IdOf},
    model::{NodeId, Session, Tab},
    prelude::*,
    protocol::{AttachRequest, Replica, SseEvent, ViewingRecord},
};

/// `ship attach <session>`. Prints the tree on stdout after every applied
/// event and status lines on stderr. The stream ends when the session is
/// removed, which prints `session removed` and exits 0; SIGINT also exits 0.
pub async fn run(client: &Client, session: IdOf<Session>) -> Result<()> {
    let request = AttachRequest {
        session,
        selection: None,
    };
    let mut events = Box::pin(client.attach(&request).await?);
    let mut observer = Observer::default();
    loop {
        let event = tokio::select! {
            event = events.next() => event,
            _ = tokio::signal::ctrl_c() => return Ok(()),
        };
        let Some(event) = event else {
            eprintln!("session removed");
            return Ok(());
        };
        let event = event.context("attach stream failed")?;
        if let SseEvent::Attached(attached) = &event {
            eprintln!("attached {} to {}", attached.attachment, session);
        }
        if observer.apply(event) {
            print!("{}", observer.render());
        }
    }
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
