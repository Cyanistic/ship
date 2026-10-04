use axum::Json;
use ship_core::{HealthResponse, PROTOCOL_VERSION};

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "health",
    responses((status = 200, description = "Ship health", body = HealthResponse))
)]
pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "ship".into(),
        protocol_version: PROTOCOL_VERSION,
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
