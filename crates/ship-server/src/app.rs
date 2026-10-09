use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use kameo::actor::ActorRef;
use ship_core::protocol::{REVISION_HEADER, Replica};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use ship_core::relay::RelayBus;

use crate::{pane::LivePanes, state::ServerState};

/// The one Axum state. Every handler extracts this; each actor ref is cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub state: ActorRef<ServerState>,
    pub bus: ActorRef<RelayBus>,
    /// Latest published replica. Each SSE stream clones this receiver.
    pub replicas: watch::Receiver<Arc<Replica>>,
    /// Every running pane's handle. SSE streams find screens through it.
    pub(crate) live: watch::Receiver<LivePanes>,
    /// Cancelled by `POST /api/v0/server/stop`; shuts down like SIGTERM.
    pub(crate) stop: CancellationToken,
    /// The state actor's revision, set by each commit before it replies.
    pub(crate) revision: Arc<AtomicU64>,
}

/// Stamps `x-ship-revision` on every response. Read after the handler, so it
/// is at least the revision of any commit the request made.
pub(crate) async fn stamp(State(app): State<AppState>, request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let revision = app.revision.load(Ordering::Acquire);
    response
        .headers_mut()
        .insert(REVISION_HEADER, revision.into());
    response
}
