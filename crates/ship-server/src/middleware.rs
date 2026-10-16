//! Request correlation and status-aware completion diagnostics.

use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};
use ship_core::{
    id::{Attachment, IdOf},
    protocol::{ATTACHMENT_HEADER, REVISION_HEADER},
};
use tower_http::request_id::RequestId;
use tracing::Instrument;

pub(crate) async fn trace_request(request: Request, next: Next) -> Response {
    let id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let attachment = request
        .headers()
        .get(ATTACHMENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<IdOf<Attachment>>().ok())
        .map(|id| id.to_string());
    let method = request.method().clone();
    // Only allowlisted, parsed metadata; never dump headers, queries or bodies.
    let path = request.uri().path().to_owned();
    let span = tracing::debug_span!("request", request_id = %id, attachment_id = attachment.as_deref(), %method, %path);
    let start = Instant::now();
    let response = next.run(request).instrument(span).await;
    let revision = response
        .headers()
        .get(REVISION_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let status = response.status().as_u16();
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    // Include context on the event itself: the debug span may be disabled.
    if response.status().is_server_error() {
        tracing::warn!(request_id = %id, attachment_id = attachment.as_deref(), revision, %method, %path, status, duration_ms, "request completed");
    } else {
        tracing::debug!(request_id = %id, attachment_id = attachment.as_deref(), revision, %method, %path, status, duration_ms, "request completed");
    }
    response
}
