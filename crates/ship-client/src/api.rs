use std::str::FromStr;

use eventsource_stream::{EventStreamError, Eventsource};
use futures_util::{Stream, StreamExt};
use indexmap::IndexMap;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use ship_core::{
    HEALTH_PATH, HealthResponse,
    id::{Attachment, Id, IdOf, Identified, Prefixed, UntaggedEither},
    model::{Named, NodeId, Pane, Session, SessionName, Tab, TabParent},
    prelude::*,
    protocol::{
        AttachRequest, Create, InputFrame, MoveTab, PaneInput, SelectRequest, SseEvent,
        SwitchSessionRequest, ViewInput, ViewingRecord,
    },
};

use url::Url;

use crate::{NO_BODY, http_error};

pub struct Client {
    pub http: reqwest::Client,
    pub url: Url,
}

/// A session ID or name, as typed on the command line. Anything containing
/// ':' is an ID, so `tab:3f2a` fails as the wrong kind rather than as a name.
#[derive(Clone, Debug)]
pub enum SessionRef {
    Id(IdOf<Session>),
    Name(SessionName),
}

impl FromStr for SessionRef {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self> {
        if value.contains(':') {
            value.parse().map(Self::Id)
        } else {
            value.parse().map(Self::Name)
        }
    }
}

/// A tab parent as typed on the command line. The tab ID comes first, matching
/// `UntaggedEither`'s try-in-order parsing, so a tab ID is never read as a session.
pub type TabParentRef = UntaggedEither<IdOf<Tab>, SessionRef>;

/// Collection path for the generic entity methods.
pub trait Resource: Identified<Id = Id<Self>> + Prefixed + DeserializeOwned {
    const COLLECTION: &'static str;
}

impl Resource for Session {
    const COLLECTION: &'static str = "/api/v0/sessions";
}

impl Resource for Tab {
    const COLLECTION: &'static str = "/api/v0/tabs";
}

impl Resource for Pane {
    const COLLECTION: &'static str = "/api/v0/panes";
}

impl Client {
    pub async fn health(&self) -> Result<HealthResponse> {
        self.request(Method::GET, HEALTH_PATH, NO_BODY, None, StatusCode::OK)
            .await
    }

    pub async fn sessions(&self) -> Result<IndexMap<IdOf<Session>, Session>> {
        self.request(
            Method::GET,
            Session::COLLECTION,
            NO_BODY,
            None,
            StatusCode::OK,
        )
        .await
    }

    pub async fn create_session(&self, name: &SessionName) -> Result<Session> {
        let body = Named { name };
        self.request(
            Method::POST,
            Session::COLLECTION,
            Some(&body),
            None,
            StatusCode::CREATED,
        )
        .await
    }

    pub async fn create_tab(&self, parent: IdOf<TabParent>, name: Option<&str>) -> Result<Tab> {
        let body = Create::<Tab> {
            parent,
            input: Named {
                name: name.map(str::to_owned).into(),
            },
        };
        self.request(
            Method::POST,
            Tab::COLLECTION,
            Some(&body),
            None,
            StatusCode::CREATED,
        )
        .await
    }

    pub async fn create_pane(&self, parent: IdOf<Tab>, input: &PaneInput) -> Result<Pane> {
        let body = Create::<Pane> {
            parent,
            input: input.clone(),
        };
        self.request(
            Method::POST,
            Pane::COLLECTION,
            Some(&body),
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

    /// `None` clears a tab or pane name; sessions reject it.
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

    /// IDs pass through untouched; a name is looked up with `GET /sessions`.
    /// The only place a session name becomes an ID.
    pub async fn resolve_session(&self, session: &SessionRef) -> Result<IdOf<Session>> {
        match session {
            SessionRef::Id(id) => Ok(*id),
            SessionRef::Name(name) => self
                .sessions()
                .await?
                .into_values()
                .find(|session| &session.name == name)
                .map(|session| session.id)
                .ok_or_else(|| err!(NotFound, "no session named '{}'", name)),
        }
    }

    pub async fn resolve_parent(&self, parent: &TabParentRef) -> Result<IdOf<TabParent>> {
        Ok(match parent {
            UntaggedEither::Left(tab) => UntaggedEither::Right(*tab),
            UntaggedEither::Right(session) => {
                UntaggedEither::Left(self.resolve_session(session).await?)
            }
        })
    }

    /// Attach-or-create for `ship attach <name>`: resolve, create if absent,
    /// and resolve again after a `409` from a concurrent creator.
    pub async fn ensure_session(&self, name: &SessionName) -> Result<IdOf<Session>> {
        let session = SessionRef::Name(name.clone());
        match self.resolve_session(&session).await {
            Err(error) if *error.code() == ErrorCode::NotFound => {}
            resolved => return resolved,
        }
        match self.create_session(name).await {
            Ok(created) => Ok(created.id),
            Err(error) if *error.code() == ErrorCode::Conflict => {
                self.resolve_session(&session).await
            }
            Err(error) => Err(error),
        }
    }

    /// Open the attach stream. The two-second timeout covers only the
    /// response head. The returned stream yields decoded events and ends on EOF.
    pub async fn attach(
        &self,
        request: &AttachRequest,
    ) -> Result<impl Stream<Item = Result<SseEvent>> + use<>> {
        let response = self
            .stream(Method::POST, "/api/v0/attach", Some(request))
            .await?;
        Ok(response
            .bytes_stream()
            .eventsource()
            .map(|event| match event {
                Ok(event) => serde_json::from_str(&event.data).map_err(
                    |error| err!(Serialization, "cannot decode attach event", @external: error),
                ),
                Err(EventStreamError::Transport(error)) => Err(http_error(error)),
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

    /// Select within the attachment's session. The attachment ID is the one
    /// the caller's own stream received, so observers never share it.
    pub async fn select(
        &self,
        attachment: IdOf<Attachment>,
        selection: NodeId,
    ) -> Result<ViewingRecord> {
        let body = SelectRequest { selection };
        self.request(
            Method::PUT,
            "/api/v0/attach/selection",
            Some(&body),
            Some(attachment),
            StatusCode::OK,
        )
        .await
    }

    /// Move the attachment to another session, selecting the session itself.
    pub async fn switch_session(
        &self,
        attachment: IdOf<Attachment>,
        session: IdOf<Session>,
    ) -> Result<ViewingRecord> {
        let body = SwitchSessionRequest { session };
        self.request(
            Method::PUT,
            "/api/v0/attach/session",
            Some(&body),
            Some(attachment),
            StatusCode::OK,
        )
        .await
    }
}
