use kameo::actor::ActorRef;

use crate::state::ServerState;

/// The one Axum state. Every handler extracts this; each actor ref is cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub state: ActorRef<ServerState>,
}
