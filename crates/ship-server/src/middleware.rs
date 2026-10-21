//! Local-only requests, request correlation and status-aware completion
//! diagnostics.

use std::{net::IpAddr, time::Instant};

use axum::{
    extract::Request,
    http::{
        header::{HOST, ORIGIN},
        uri::Authority,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use ship_core::{
    AppError, err,
    id::{Attachment, IdOf},
    protocol::{ATTACHMENT_HEADER, REVISION_HEADER},
};
use tower_http::request_id::RequestId;
use tracing::Instrument;

/// Refuses anything a web page could make a browser send, so only programs on
/// this machine reach the server. DNS rebinding arrives with the page's own
/// host; a page posting straight to the loopback address carries `Origin`.
pub(crate) async fn local_only(request: Request, next: Next) -> Response {
    if request.headers().contains_key(ORIGIN) {
        return err!(
            Forbidden,
            "the server doesn't answer requests from web pages"
        )
        .into_response();
    }
    if !names_loopback(&request) {
        return err!(
            Forbidden,
            "the server answers only requests addressed to localhost"
        )
        .into_response();
    }
    next.run(request).await
}

/// The HTTP/2 authority or else the HTTP/1 `Host` header is `localhost` or a
/// loopback address, with any port.
fn names_loopback(request: &Request) -> bool {
    let authority = request.uri().authority().cloned().or_else(|| {
        let host = request.headers().get(HOST)?.to_str().ok()?;
        host.parse::<Authority>().ok()
    });
    let Some(authority) = authority else {
        return false;
    };
    let host = authority.host();
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

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
    let error = response
        .extensions()
        .get::<AppError>()
        .map(ToString::to_string);
    // Include context on the event itself: the debug span may be disabled.
    if response.status().is_server_error() {
        tracing::warn!(request_id = %id, attachment_id = attachment.as_deref(), revision, %method, %path, status, duration_ms, error = error.as_deref(), "request completed");
    } else {
        tracing::debug!(request_id = %id, attachment_id = attachment.as_deref(), revision, %method, %path, status, duration_ms, error = error.as_deref(), "request completed");
    }
    response
}
