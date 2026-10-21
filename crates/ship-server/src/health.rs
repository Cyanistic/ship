use axum::{Json, extract::State};
use ship_core::{HealthResponse, PROTOCOL_VERSION, Result, SERVICE};

use crate::AppState;

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "health",
    responses((status = 200, description = "Ship health", body = HealthResponse))
)]
pub(crate) async fn health(State(app): State<AppState>) -> Result<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        service: SERVICE.into(),
        protocol_version: PROTOCOL_VERSION,
        version: env!("CARGO_PKG_VERSION").into(),
        log: (app.log)(None)?.file,
    }))
}
