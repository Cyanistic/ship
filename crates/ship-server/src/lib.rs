//! Loopback HTTP listener, health route, session, tab and pane API and
//! attach stream.

mod app;
mod attach;
mod health;
mod pane;
mod routes;
mod state;

use std::{
    future::{Future, IntoFuture},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

pub use app::AppState;
use axum::Router;
use kameo::{
    actor::{ActorRef, Spawn},
    mailbox,
};
use ship_core::{prelude::*, protocol::Replica, relay};
use tokio::{
    net::TcpListener,
    sync::{mpsc, watch},
};
use tower_http::{
    compression::{
        CompressionLayer, Predicate,
        predicate::{NotForContentType, SizeAbove},
    },
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn loopback_addr(port: u16) -> Result<SocketAddr> {
    if port == 0 {
        return Err(err!(Configuration, "server port must not be zero"));
    }
    Ok(SocketAddr::from(([127, 0, 0, 1], port)))
}

/// Document metadata. Expanding the derive here takes title, version and
/// description from ship-server's manifest; paths and schemas come from
/// `routes!`. Schemas reached only through a hand-written `$ref`, such as
/// `Replica::viewers` or the flattened `PaneSpec` in `Create_Pane`, are
/// registered here.
#[derive(OpenApi)]
#[openapi(components(schemas(ship_core::protocol::ViewingRecord, ship_core::protocol::PaneSpec)))]
struct ApiDoc;

/// One `.routes(routes!(handler))` per handler. Registrations on the same path
/// merge, so handlers need not be grouped by path.
macro_rules! api_routes {
    ($router:expr, $($handler:path),+ $(,)?) => {
        $router$(.routes(routes!($handler)))+
    };
}

fn api_router() -> OpenApiRouter<AppState> {
    api_routes!(
        OpenApiRouter::with_openapi(ApiDoc::openapi()),
        health::health,
        routes::list_sessions,
        routes::create_session,
        routes::get_session,
        routes::rename_session,
        routes::remove_session,
        routes::create_tab,
        routes::get_tab,
        routes::rename_tab,
        routes::remove_tab,
        routes::move_tab,
        routes::create_pane,
        routes::get_pane,
        routes::rename_pane,
        routes::remove_pane,
        attach::attach,
        attach::select,
        attach::switch_session,
    )
}

/// Construct the route-built API description without starting the server.
pub fn openapi() -> utoipa::openapi::OpenApi {
    api_router().split_for_parts().1
}

pub fn router(app: AppState) -> Router {
    // The default predicate minus `NotForContentType::SSE`, so attach streams
    // are compressed too.
    let compression = CompressionLayer::new().zstd(true).gzip(true).compress_when(
        SizeAbove::new(32)
            .and(NotForContentType::GRPC)
            .and(NotForContentType::IMAGES),
    );
    api_router().layer(compression).layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<axum::body::Body>| {
                tracing::info_span!("request", method = %request.method(), path = request.uri().path())
            })
            .on_response(|response: &axum::http::Response<axum::body::Body>, duration: Duration, _span: &tracing::Span| {
                tracing::info!(status = response.status().as_u16(), duration_ms = duration.as_secs_f64() * 1000.0, "request completed");
            }),
    ).split_for_parts().0.with_state(app)
}

pub async fn serve(
    address: SocketAddr,
    shutdown: impl Future<Output = Result<()>> + Send + 'static,
) -> Result<()> {
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(err!(
            Configuration,
            "server requires a loopback address and nonzero port"
        ));
    }
    let listener = TcpListener::bind(address).await.map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::AddrInUse {
            ErrorCode::Conflict
        } else {
            ErrorCode::Io
        };
        err!(code, "cannot bind server at {}", address, @external: error)
    })?;
    tracing::info!(%address, pid = std::process::id(), protocol_version = ship_core::PROTOCOL_VERSION, "server listening");
    // Axum awaits shutdown directly. This channel only observes its result so
    // the outer task can propagate errors and bound draining after signal receipt.
    let (result_tx, mut result_rx) = watch::channel(None);
    let bus = relay::RelayBus::spawn_with_mailbox(relay::RelayBus::default(), mailbox::bounded(64));
    let state = state::ServerState::new(bus.clone());
    let (replica_tx, replicas) = watch::channel(state.replica());
    bus.ask(relay::Subscribe::<Arc<Replica>> {
        sink: Box::new(replica_tx),
    })
    .await
    .map_err(|error| err!(Internal, "cannot subscribe replica stream", @external: error))?;
    let state = state::ServerState::spawn(state);
    forward_pane_events(&bus, state.clone()).await?;
    let actors = (state.clone(), bus.clone());
    let app = AppState {
        state,
        bus,
        replicas,
    };
    let serving = axum::serve(listener, router(app))
        .with_graceful_shutdown(async move {
            let (state, bus) = actors;
            let result = tokio::select! {
                result = shutdown => result,
                error = actor_stopped(&state, &bus) => Err(error),
            };
            result_tx.send_replace(Some(result));
            stop_actors(&state, &bus).await;
        })
        .into_future();
    tokio::pin!(serving);
    tokio::select! {
        result = &mut serving => result.map_err(|error| err!(Io, "server failed", @external: error))?,
        changed = result_rx.changed() => {
            changed.map_err(|error| err!(Internal, "shutdown outcome unavailable", @external: error))?;
            tokio::time::timeout(Duration::from_secs(5), &mut serving)
                .await
                .map_err(|_| err!(Internal, "server shutdown exceeded five seconds"))?
                .map_err(|error| err!(Io, "server shutdown failed", @external: error))?;
        }
    }
    result_rx.borrow().clone().unwrap_or(Ok(()))
}

/// Pane titles and exits reach the state actor through an unbounded sink and
/// an awaited `tell`, so an exit status waits rather than drops.
async fn forward_pane_events(
    bus: &ActorRef<relay::RelayBus>,
    state: ActorRef<state::ServerState>,
) -> Result<()> {
    let (events_tx, mut events) = mpsc::unbounded_channel::<pane::PaneEvent>();
    bus.ask(relay::Subscribe {
        sink: Box::new(events_tx),
    })
    .await
    .map_err(|error| err!(Internal, "cannot subscribe pane events", @external: error))?;
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            if state.tell(event).await.is_err() {
                break;
            }
        }
    });
    Ok(())
}

/// Resolves when either actor stops on its own, such as after a panic. A
/// server without them would keep serving stale observers, so this begins
/// shutdown with an error.
async fn actor_stopped(
    state: &ActorRef<state::ServerState>,
    bus: &ActorRef<relay::RelayBus>,
) -> AppError {
    let (actor, reason) = tokio::select! {
        reason = state.wait_for_shutdown_result() => ("state", reason),
        reason = bus.wait_for_shutdown_result() => ("relay", reason),
    };
    let reason = match reason {
        Ok(reason) => reason.to_string(),
        Err(error) => error.to_string(),
    };
    err!(Internal, "{} actor stopped: {}", actor, reason)
}

/// Stops the state actor, then the relay. The state actor goes first so its
/// queued commits still publish. Dropping the relay drops the replica watch
/// sender, which ends every attach stream with `serverShutdown` so Axum's
/// drain can finish. Stopping an actor that already stopped is a no-op.
/// `wait_for_shutdown_result`, unlike `wait_for_shutdown`, also waits for the
/// state actor's `on_stop`, which tears down every pane.
async fn stop_actors(state: &ActorRef<state::ServerState>, bus: &ActorRef<relay::RelayBus>) {
    state.stop_gracefully().await.ok();
    state.wait_for_shutdown_result().await.ok();
    bus.stop_gracefully().await.ok();
    bus.wait_for_shutdown().await;
}
