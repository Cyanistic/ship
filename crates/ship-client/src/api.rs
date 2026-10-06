use std::str::FromStr;

use indexmap::IndexMap;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use ship_core::{
    HEALTH_PATH, HealthResponse,
    id::{Id, IdOf, Identified, Prefixed, UntaggedEither},
    model::{Named, Pane, Session, SessionName, Tab, TabParent},
    prelude::*,
    protocol::{Create, MoveTab},
};

use url::Url;

use crate::NO_BODY;

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

    pub async fn create_tab(&self, parent: IdOf<TabParent>, name: &str) -> Result<Tab> {
        let body = Create::<Tab> {
            parent,
            input: Named {
                name: name.to_owned(),
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

    pub async fn create_pane(&self, parent: IdOf<Tab>, name: &str) -> Result<Pane> {
        let body = Create::<Pane> {
            parent,
            input: Named {
                name: name.to_owned(),
            },
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

    pub async fn rename<T: Resource>(&self, id: IdOf<T>, name: &str) -> Result<T> {
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
}
