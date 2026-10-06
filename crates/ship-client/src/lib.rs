//! Shared-type HTTP client; identity validation and startup belong to the application.

mod api;

use std::{error::Error, io, time::Duration};

pub use api::{Client, Resource, SessionRef, TabParentRef};
use reqwest::{Error as HttpError, Method, RequestBuilder, Response, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use ship_core::{
    id::{Attachment, IdOf},
    prelude::*,
    protocol::ATTACHMENT_HEADER,
};
use url::Url;

/// `request` body argument for bodiless requests.
const NO_BODY: Option<&()> = None;

const TIMEOUT: Duration = Duration::from_secs(2);

impl Client {
    /// Send one request and decode the expected success body. The two-second
    /// timeout covers the whole exchange.
    async fn request<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        attachment: Option<IdOf<Attachment>>,
        expected_status: StatusCode,
    ) -> Result<T> {
        let (request, safe) = self.build(method, path, body, attachment);
        let exchange = async {
            let response = open(request, expected_status).await?;
            let bytes = response.bytes().await.map_err(http_error)?;
            // A 204 has no body; decode it as JSON null so `()` succeeds.
            let bytes: &[u8] = if bytes.is_empty() { b"null" } else { &bytes };
            serde_json::from_slice(bytes)
                .map_err(|error| err!(Serialization, "cannot decode response", @external: error))
        };
        tokio::time::timeout(TIMEOUT, exchange)
            .await
            .unwrap_or_else(|_| Err(err!(Network, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }

    /// Start a streaming request. The two-second timeout covers only the
    /// response head (and an error body); the returned body is unbounded.
    async fn stream<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let (request, safe) = self.build(method, path, body, None);
        tokio::time::timeout(TIMEOUT, open(request, StatusCode::OK))
            .await
            .unwrap_or_else(|_| Err(err!(Network, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }

    /// The request, and its URL without credentials for error messages.
    fn build<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        attachment: Option<IdOf<Attachment>>,
    ) -> (RequestBuilder, Url) {
        let mut url = self.url.clone();
        url.set_path(path);
        url.set_query(None);
        url.set_fragment(None);
        let mut safe = url.clone();
        let _ = safe.set_username("");
        let _ = safe.set_password(None);

        let mut request = self.http.request(method, url);
        if let Some(body) = body {
            request = request.json(body);
        }
        if let Some(attachment) = attachment {
            request = request.header(ATTACHMENT_HEADER, attachment.to_string());
        }
        (request, safe)
    }
}

/// Send and check the status. Any other status returns the server's
/// `AppError` when the body is one, so server messages reach the caller;
/// otherwise the status and body text.
async fn open(request: RequestBuilder, expected_status: StatusCode) -> Result<Response> {
    let response = request.send().await.map_err(http_error)?;
    let status = response.status();
    if status == expected_status {
        return Ok(response);
    }
    let bytes = response.bytes().await.map_err(http_error)?;
    Err(
        serde_json::from_slice::<AppError>(&bytes).unwrap_or_else(|_| {
            let text = String::from_utf8_lossy(&bytes);
            let text = text.trim();
            err!(
                ErrorCode::UpstreamHttpStatus(status.as_u16()),
                "expected HTTP {}, received HTTP {}{}{}",
                expected_status.as_u16(),
                status.as_u16(),
                if text.is_empty() { "" } else { ": " },
                text
            )
        }),
    )
}

fn http_error(error: HttpError) -> AppError {
    // Classify retained foreign causes before AppError snapshots them as text.
    // Body/decode failures cannot establish an absent listener.
    let connection_refused = error.is_connect()
        && std::iter::successors(error.source(), |source| (*source).source()).any(|source| {
            source
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::ConnectionRefused)
        });
    let code = if connection_refused {
        ErrorCode::ConnectionRefused
    } else if error.is_decode() {
        ErrorCode::Serialization
    } else {
        ErrorCode::Network
    };
    AppError::external(code, error.without_url())
}
