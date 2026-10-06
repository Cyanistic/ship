use std::str::FromStr;

use indexmap::IndexMap;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use ship_core::{
    HEALTH_PATH, HealthResponse,
    id::{Id, IdOf, Identified, Prefixed},
    model::{Named, Session, SessionName},
    prelude::*,
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

/// Collection path for the generic entity methods.
pub trait Resource: Identified<Id = Id<Self>> + Prefixed + DeserializeOwned {
    const COLLECTION: &'static str;
}

impl Resource for Session {
    const COLLECTION: &'static str = "/api/v0/sessions";
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
}
