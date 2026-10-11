use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ship_core::{AppError, Result, err, id::*, model::*, protocol::*};

use tokio::sync::oneshot;

use crate::{AppState, pane::PaneCommand, state::*};

type App = State<AppState>;

#[utoipa::path(
    get,
    path = "/api/v0/tabs",
    operation_id = "list_tabs",
    responses(
        (status = 200, description = "Top-level tabs keyed by ID, in order, with their descendants", body = IndexMap<String, Tab>),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn list_tabs(State(app): App) -> Result<Response> {
    let value = app.state.ask(ListTabs).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/tabs",
    operation_id = "create_tab",
    request_body = CreateTab,
    responses(
        (status = 201, body = Tab),
        (status = 404, description = "Parent or sibling not found", body = AppError),
        (status = 422, description = "Malformed destination, name or starter"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn create_tab(State(app): App, Json(body): Json<CreateTab>) -> Result<Response> {
    let tab = app.state.ask(body).await?;
    Ok((StatusCode::CREATED, Json(tab)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/tabs/{id}",
    operation_id = "get_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    responses(
        (status = 200, body = Tab),
        (status = 400, description = "Malformed tab ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Result<Response> {
    let value = app.state.ask(Get::<Tab>(id)).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v0/tabs/{id}",
    operation_id = "rename_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    request_body = Named<OptionalName>,
    responses(
        (status = 200, body = Tab),
        (status = 400, description = "Malformed tab ID"),
        (status = 404, body = AppError),
        (status = 422, description = "Malformed name"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn rename_tab(
    State(app): App,
    Path(id): Path<IdOf<Tab>>,
    Json(body): Json<Named<OptionalName>>,
) -> Result<Response> {
    let value = app
        .state
        .ask(Rename::<Tab> {
            id,
            name: body.name,
        })
        .await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/tabs/{id}",
    operation_id = "remove_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    responses(
        (status = 204, description = "Tab and its descendants removed"),
        (status = 400, description = "Malformed tab ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn remove_tab(State(app): App, Path(id): Path<IdOf<Tab>>) -> Result<Response> {
    app.state.ask(Remove::<Tab>(id)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/tabs/{id}/move",
    operation_id = "move_tab",
    params(("id" = String, Path, description = "Tab ID, e.g. tab:3f2a...")),
    request_body = MoveTab,
    responses(
        (status = 200, description = "The moved tab with its subtree", body = Tab),
        (status = 400, description = "Malformed tab ID"),
        (status = 404, description = "Tab, parent or sibling not found", body = AppError),
        (status = 422, description = "Move into itself or a descendant, or a malformed body", body = AppError),
        (status = 503, body = AppError),
    ),
)]
/// The body is read as a JSON value first: serde_json calls a map without
/// exactly one destination key a syntax error, which `Json<MoveTab>` would
/// answer with 400 rather than 422.
pub(crate) async fn move_tab(
    State(app): App,
    Path(id): Path<IdOf<Tab>>,
    Json(to): Json<serde_json::Value>,
) -> Result<Response> {
    let to: MoveTab = serde_json::from_value(to)
        .map_err(|error| err!(InvalidStructure, "malformed move body", @external: error))?;
    let value = app.state.ask(Move { id, to }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes",
    operation_id = "create_pane",
    request_body = CreatePane,
    responses(
        (status = 201, body = Pane),
        (status = 400, description = "Malformed JSON, or invalid program or starting directory"),
        (status = 404, description = "Tab or anchor pane not found", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn create_pane(State(app): App, Json(body): Json<CreatePane>) -> Result<Response> {
    let pane = app.state.ask(body).await?;
    Ok((StatusCode::CREATED, Json(pane)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/panes/{id}",
    operation_id = "get_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    responses(
        (status = 200, body = Pane),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_pane(State(app): App, Path(id): Path<IdOf<Pane>>) -> Result<Response> {
    let value = app.state.ask(Get::<Pane>(id)).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v0/panes/{id}",
    operation_id = "rename_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = Named<OptionalName>,
    responses(
        (status = 200, body = Pane),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, body = AppError),
        (status = 422, description = "Malformed name"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn rename_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Json(body): Json<Named<OptionalName>>,
) -> Result<Response> {
    let value = app
        .state
        .ask(Rename::<Pane> {
            id,
            name: body.name,
        })
        .await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/panes/{id}",
    operation_id = "remove_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    responses(
        (status = 204, description = "Pane removed"),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn remove_pane(State(app): App, Path(id): Path<IdOf<Pane>>) -> Result<Response> {
    app.state.ask(Remove::<Pane>(id)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/zoom",
    operation_id = "zoom_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = ZoomPane,
    responses(
        (status = 200, description = "The pane's tab", body = Tab),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, body = AppError),
        (status = 422, description = "Malformed body"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn zoom_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Json(zoom): Json<ZoomPane>,
) -> Result<Response> {
    let value = app.state.ask(Zoom { id, zoom }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/swap",
    operation_id = "swap_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = SwapPane,
    responses(
        (status = 200, description = "The panes' tab", body = Tab),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, description = "Either pane not found", body = AppError),
        (status = 422, description = "Panes in different tabs, the same pane twice, or a malformed body", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn swap_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Json(swap): Json<SwapPane>,
) -> Result<Response> {
    let value = app.state.ask(Swap { id, swap }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/resize",
    operation_id = "resize_pane",
    params(("id" = String, Path, description = "Pane ID, e.g. pane:3f2a...")),
    request_body = ResizePane,
    responses(
        (status = 200, description = "The pane's tab, unchanged when the edge can't move", body = Tab),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, body = AppError),
        (status = 422, description = "Malformed body"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn resize_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Json(resize): Json<ResizePane>,
) -> Result<Response> {
    let value = app.state.ask(Resize { id, resize }).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/panes/{id}/scroll",
    operation_id = "scroll_pane",
    params(("id" = String, Path, description = "Pane ID")),
    request_body = ScrollPane,
    responses(
        (status = 204, description = "Scroll queued, including for exited panes"),
        (status = 400, description = "Malformed pane ID"),
        (status = 404, body = AppError),
        (status = 422, description = "Malformed body"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn scroll_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Json(body): Json<ScrollPane>,
) -> Result<StatusCode> {
    let live = app.live.borrow();
    let pane = live
        .get(&id)
        .ok_or_else(|| err!(NotFound, "pane {} not found", id))?;
    pane.commands
        .send(PaneCommand::Scroll(body))
        .map_err(|_| err!(Internal, "pane {} task ended", id))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v0/panes/{id}/text",
    operation_id = "read_pane",
    params(
        ("id" = String, Path, description = "Pane ID"),
        ("lines" = Option<u32>, Query, description = "Last N rows including history"),
        ("unwrap" = Option<bool>, Query, description = "Join soft-wrapped lines"),
        ("ansi" = Option<bool>, Query, description = "Keep colors and styles"),
    ),
    responses(
        (status = 200, body = String, content_type = "text/plain"),
        (status = 400, description = "Malformed pane ID or query"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn read_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Query(read): Query<ReadPane>,
) -> Result<String> {
    let (reply, text) = oneshot::channel();
    {
        let live = app.live.borrow();
        let pane = live
            .get(&id)
            .ok_or_else(|| err!(NotFound, "pane {} not found", id))?;
        pane.commands
            .send(PaneCommand::Read(read, reply))
            .map_err(|_| err!(Internal, "pane {} task ended", id))?;
    }
    text.await
        .map_err(|_| err!(Internal, "pane {} read failed", id))
}

#[utoipa::path(
    post,
    path = "/api/v0/server/stop",
    operation_id = "stop_server",
    responses(
        (status = 202, description = "Shutdown started, as on SIGTERM. The server keeps \
            answering until its panes are torn down, then exits."),
    ),
)]
pub(crate) async fn stop_server(State(app): App) -> StatusCode {
    app.stop.cancel();
    StatusCode::ACCEPTED
}
