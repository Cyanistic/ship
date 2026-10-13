//! Each attachment's view, readable and selectable by anyone holding its ID.

use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use ship_core::{
    AppError, Result,
    id::{Attachment, IdOf},
    protocol::{SelectView, ViewingRecord},
};

use crate::{
    AppState,
    state::{GetView, ListViews, Select},
};

#[utoipa::path(
    get,
    path = "/api/v0/views",
    operation_id = "list_views",
    responses(
        (status = 200, description = "Every attachment's view keyed by ID, in attach order", body = IndexMap<String, ViewingRecord>),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn list_views(State(app): State<AppState>) -> Result<Response> {
    let value = app.state.ask(ListViews).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v0/views/{attachment}",
    operation_id = "get_view",
    params(("attachment" = String, Path, description = "Attachment ID from ship view list")),
    responses(
        (status = 200, body = ViewingRecord),
        (status = 400, description = "Malformed attachment ID"),
        (status = 404, body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn get_view(
    State(app): State<AppState>,
    Path(attachment): Path<IdOf<Attachment>>,
) -> Result<Response> {
    let value = app.state.ask(GetView(attachment)).await?;
    Ok(Json(value).into_response())
}

#[utoipa::path(
    put,
    path = "/api/v0/views/{attachment}/selection",
    operation_id = "select_view",
    params(("attachment" = String, Path, description = "Attachment ID from ship view list")),
    request_body = SelectView,
    responses(
        (status = 200, description = "The stored view; selecting a zoom-hidden pane unzooms its tab", body = ViewingRecord),
        (status = 400, description = "Malformed attachment ID or JSON"),
        (status = 404, description = "Attachment or node not found; nothing changed", body = AppError),
        (status = 422, description = "Malformed selection"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn select(
    State(app): State<AppState>,
    Path(attachment): Path<IdOf<Attachment>>,
    Json(body): Json<SelectView>,
) -> Result<Response> {
    let value = app
        .state
        .ask(Select {
            attachment,
            selection: body.selection,
        })
        .await?;
    Ok(Json(value).into_response())
}
