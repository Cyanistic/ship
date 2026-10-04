use reqwest::{Method, StatusCode};
use ship_core::{HEALTH_PATH, HealthResponse, prelude::*};

use url::Url;

pub struct Client {
    pub http: reqwest::Client,
    pub url: Url,
}

impl Client {
    pub async fn health(&self) -> Result<HealthResponse> {
        self.request(Method::GET, HEALTH_PATH, StatusCode::OK).await
    }
}
