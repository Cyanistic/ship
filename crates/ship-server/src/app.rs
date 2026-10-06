use std::sync::Arc;

use kameo::actor::ActorRef;
use ship_core::protocol::Replica;
use tokio::sync::watch;

use ship_core::relay::RelayBus;

use crate::state::ServerState;

/// The one Axum state. Every handler extracts this; each actor ref is cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub state: ActorRef<ServerState>,
    pub bus: ActorRef<RelayBus>,
    /// Latest published replica. Each SSE stream clones this receiver.
    pub replicas: watch::Receiver<Arc<Replica>>,
}
