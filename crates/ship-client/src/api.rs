use std::{
    future::ready,
    sync::atomic::{AtomicU64, Ordering},
};

use futures_util::{Stream, StreamExt, TryStreamExt};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use ship_core::{
    HEALTH_PATH, HealthResponse,
    id::{Attachment, Id, IdOf, Identified, Prefixed},
    model::{Named, Pane, Tab},
    prelude::*,
    protocol::{
        AttachRequest, CreatePane, CreateTab, InputFrame, MoveTab, SseEvent, ViewInput,
        ViewingRecord,
    },
    tree::Tabs,
};
use sse_stream::SseByteStream;
use url::Url;

use crate::{NO_BODY, http_error};

pub struct Client {
    pub http: reqwest::Client,
    pub url: Url,
    /// Highest `x-ship-revision` a response carried; 0 before any.
    written: AtomicU64,
}

/// Collection path for the generic entity methods.
pub trait Resource: Identified<Id = Id<Self>> + Prefixed + DeserializeOwned {
    const COLLECTION: &'static str;
}

impl Resource for Tab {
    const COLLECTION: &'static str = "/api/v0/tabs";
}

impl Resource for Pane {
    const COLLECTION: &'static str = "/api/v0/panes";
}

impl Client {
    pub fn new(http: reqwest::Client, url: Url) -> Self {
        Self {
            http,
            url,
            written: AtomicU64::new(0),
        }
    }

    /// The highest revision a response reported. A replica at or past it
    /// includes every change this client's requests made.
    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Acquire)
    }

    /// Raise `written` to `revision`; a late response never lowers it.
    pub(crate) fn wrote(&self, revision: u64) {
        self.written.fetch_max(revision, Ordering::AcqRel);
    }

    /// Back to 0, for a server that may have restarted and counts afresh.
    pub fn forget(&self) {
        self.written.store(0, Ordering::Release);
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        self.request(Method::GET, HEALTH_PATH, NO_BODY, None, StatusCode::OK)
            .await
    }

    /// Starts the server's shutdown; it exits once its panes are torn down.
    pub async fn stop_server(&self) -> Result<()> {
        self.request(
            Method::POST,
            "/api/v0/server/stop",
            NO_BODY,
            None,
            StatusCode::ACCEPTED,
        )
        .await
    }

    /// Top-level tabs with their descendants, in order. Each viewed tab
    /// carries server-computed geometry; unviewed tabs omit it.
    pub async fn tabs(&self) -> Result<Tabs> {
        self.request(Method::GET, Tab::COLLECTION, NO_BODY, None, StatusCode::OK)
            .await
    }

    pub async fn create_tab(&self, body: &CreateTab) -> Result<Tab> {
        self.request(
            Method::POST,
            Tab::COLLECTION,
            Some(body),
            None,
            StatusCode::CREATED,
        )
        .await
    }

    pub async fn create_pane(&self, body: &CreatePane) -> Result<Pane> {
        self.request(
            Method::POST,
            Pane::COLLECTION,
            Some(body),
            None,
            StatusCode::CREATED,
        )
        .await
    }

    pub async fn get<T: Resource>(&self, id: IdOf<T>) -> Result<T> {
        let path = format!("{}/{id}", T::COLLECTION);
        self.request(Method::GET, &path, NO_BODY, None, StatusCode::OK)
            .await
    }

    /// `None` clears the name.
    pub async fn rename<T: Resource>(&self, id: IdOf<T>, name: Option<&str>) -> Result<T> {
        let path = format!("{}/{id}", T::COLLECTION);
        let body = Named { name };
        self.request(Method::PATCH, &path, Some(&body), None, StatusCode::OK)
            .await
    }

    pub async fn remove<T: Resource>(&self, id: IdOf<T>) -> Result<()> {
        let path = format!("{}/{id}", T::COLLECTION);
        self.request(Method::DELETE, &path, NO_BODY, None, StatusCode::NO_CONTENT)
            .await
    }

    pub async fn move_tab(&self, id: IdOf<Tab>, to: &MoveTab) -> Result<Tab> {
        let path = format!("{}/{id}/move", Tab::COLLECTION);
        self.request(Method::POST, &path, Some(to), None, StatusCode::OK)
            .await
    }

    /// Open the attach stream. The two-second timeout covers only the
    /// response head. The returned stream yields decoded events and ends on EOF.
    ///
    /// `sse-stream` scans each received chunk once; `eventsource-stream`
    /// rescanned a partial line on every chunk, which was quadratic in the size
    /// of a screen event. Blocks without data are skipped.
    pub async fn attach(
        &self,
        request: &AttachRequest,
    ) -> Result<impl Stream<Item = Result<SseEvent>> + use<>> {
        let response = self
            .stream(Method::POST, "/api/v0/attach", Some(request))
            .await?;
        Ok(SseByteStream::new(response.bytes_stream())
            .try_filter_map(|block| ready(Ok(block.data)))
            .map(|data| match data {
                Ok(data) => serde_json::from_str(&data).map_err(
                    |error| err!(Serialization, "cannot decode attach event", @external: error),
                ),
                Err(sse_stream::Error::Body(error)) => match error.downcast::<reqwest::Error>() {
                    Ok(error) => Err(http_error(*error)),
                    Err(error) => {
                        Err(err!(Serialization, "malformed attach stream", @external: error))
                    }
                },
                Err(error) => Err(err!(Serialization, "malformed attach stream", @external: error)),
            }))
    }

    /// Stream `frames` to the attachment's panes in one POST, until `frames`
    /// ends or the server answers. Only the connect timeout applies.
    pub async fn input(
        &self,
        attachment: IdOf<Attachment>,
        frames: impl Stream<Item = InputFrame> + Send + 'static,
    ) -> Result<()> {
        self.send_stream(
            "/api/v0/attach/input",
            attachment,
            frames,
            StatusCode::NO_CONTENT,
        )
        .await
    }

    /// Replace the attachment's view; returns the record as stored.
    pub async fn set_view(
        &self,
        attachment: IdOf<Attachment>,
        view: &ViewInput,
    ) -> Result<ViewingRecord> {
        self.request(
            Method::PUT,
            "/api/v0/attach/view",
            Some(view),
            Some(attachment),
            StatusCode::OK,
        )
        .await
    }
}
