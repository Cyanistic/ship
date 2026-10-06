use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ship_core::{AppError, Result, id::*, model::*, protocol::*};

use crate::{AppState, state::*};

type App = State<AppState>;

#[utoipa::path(
    get,
    path = "/api/v0/sessions",
    operation_id = "list_sessions",
    responses(
        (status = 200, description = "Sessions keyed by ID, in creation order", body = IndexMap<String, Session>),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn list_sessions(State(app): App) -> Result<Response> {
    let value = app.state.ask(ListSessions).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v0/sessions",
    operation_id = "create_session",
    request_body = Named<SessionName>,
    responses(
        (status = 201, body = Session),
        (status = 409, description = "Session name already exists", body = AppError),
        (status = 422, description = "Malformed name"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn create_session(
    State(app): App,
    Json(body): Json<Named<SessionName>>,
) -> Result<Response> {
    let session = app
        .state
        .ask(Create::<Session> {
            parent: (),
            input: body,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(session)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/sessions/{id}",
    operation_id = "get_session",
    params(("id" = String, Path, description = "Session ID, e.g. session:3f2a...")),
    responses(
        (status = 200, body = Session),
        (status = 400, description = "Malformed session ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_session(
    State(app): App,
    Path(id): Path<IdOf<Session>>,
) -> Result<Response> {
    let value = app.state.ask(Get::<Session>(id)).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v0/sessions/{id}",
    operation_id = "rename_session",
    params(("id" = String, Path, description = "Session ID, e.g. session:3f2a...")),
    request_body = Named<SessionName>,
    responses(
        (status = 200, body = Session),
        (status = 400, description = "Malformed session ID"),
        (status = 404, body = AppError),
        (status = 409, description = "Session name already exists", body = AppError),
        (status = 422, description = "Malformed name"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn rename_session(
    State(app): App,
    Path(id): Path<IdOf<Session>>,
    Json(body): Json<Named<SessionName>>,
) -> Result<Response> {
    let value = app
        .state
        .ask(Rename::<Session> {
            id,
            name: body.name.to_string(),
        })
        .await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v0/sessions/{id}",
    operation_id = "remove_session",
    params(("id" = String, Path, description = "Session ID, e.g. session:3f2a...")),
    responses(
        (status = 204, description = "Session and its contents removed"),
        (status = 400, description = "Malformed session ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn remove_session(
    State(app): App,
    Path(id): Path<IdOf<Session>>,
) -> Result<Response> {
    app.state.ask(Remove::<Session>(id)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
