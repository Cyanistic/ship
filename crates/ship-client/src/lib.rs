//! Shared-type HTTP client; identity validation and startup belong to the application.

mod api;

use std::{error::Error, io, time::Duration};

pub use api::Client;
use reqwest::{Error as HttpError, Method, StatusCode};
use serde::de::DeserializeOwned;
use ship_core::prelude::*;

impl Client {
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        expected_status: StatusCode,
    ) -> Result<T> {
        let mut url = self.url.clone();
        url.set_path(path);
        url.set_query(None);
        url.set_fragment(None);
        let mut safe = url.clone();
        let _ = safe.set_username("");
        let _ = safe.set_password(None);

        let request = async {
            let response = self
                .http
                .request(method, url)
                .send()
                .await
                .map_err(http_error)?;
            if response.status() != expected_status {
                return Err(err!(
                    ErrorCode::UpstreamHttpStatus(response.status().as_u16()),
                    "expected HTTP {}, received HTTP {}",
                    expected_status.as_u16(),
                    response.status().as_u16()
                ));
            }
            response.json().await.map_err(http_error)
        };
        tokio::time::timeout(Duration::from_secs(2), request)
            .await
            .unwrap_or_else(|_| Err(err!(Network, "HTTP request timed out")))
            .context(format!("request to {safe} failed"))
    }
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
