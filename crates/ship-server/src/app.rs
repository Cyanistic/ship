use std::sync::Arc;

use kameo::actor::ActorRef;
use ship_core::protocol::Replica;
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
}
