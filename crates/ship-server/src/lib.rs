//! Loopback HTTP listener, health route, session, tab and pane API and
//! attach stream.

mod app;
mod attach;
mod health;
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
use kameo::{actor::Spawn, mailbox};
use ship_core::{prelude::*, protocol::Replica, relay};
use tokio::{net::TcpListener, sync::watch};
use tower_http::trace::TraceLayer;
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
/// `Replica::viewers`, are registered here.
#[derive(OpenApi)]
#[openapi(components(schemas(ship_core::protocol::ViewingRecord)))]
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
    api_router().layer(
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
    let app = AppState {
        state: state::ServerState::spawn(state),
        bus,
        replicas,
    };
    let serving = axum::serve(listener, router(app))
        .with_graceful_shutdown(async move {
            result_tx.send_replace(Some(shutdown.await));
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
