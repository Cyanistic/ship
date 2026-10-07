use std::{future::ready, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::request::Parts,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures_util::{StreamExt, stream};
use kameo::actor::ActorRef;
use ship_core::{AppError, Result, err, id::*, model::Pane, protocol::*, screen::Screen};
use tokio::sync::watch;
use tokio_stream::{StreamMap, wrappers::WatchStream};

use crate::{
    AppState,
    pane::LivePanes,
    state::{Attach, Detach, Select, ServerState, SetView, SwitchSession, tree},
};

/// Created before asking the state actor, so cancellation at any point still
/// detaches the right ID. Drop only initiates the detach; the actor finishes
/// it asynchronously, and a send failure after the actor has stopped is ignored.
struct AttachmentGuard {
    attachment: IdOf<Attachment>,
    state: ActorRef<ServerState>,
}

impl Drop for AttachmentGuard {
    fn drop(&mut self) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let (attachment, state) = (self.attachment, self.state.clone());
        runtime.spawn(async move {
            state.tell(Detach(attachment)).await.ok();
        });
    }
}

#[utoipa::path(
    post,
    path = "/api/v0/attach",
    operation_id = "attach",
    request_body = AttachRequest,
    responses(
        (status = 200, content_type = "text/event-stream", body = SseEvent,
            description = "Server-sent events whose `data:` lines are `SseEvent` JSON: \
                `attached` first, then `state` for each newer replica and `screen` for each \
                changed screen of a pane in the viewed tab, then `ended` with the reason \
                when the attachment's session is removed or the server shuts down. Viewing \
                a tab sends each of its panes' current screens."),
        (status = 404, description = "Session not found", body = AppError),
        (status = 422, description = "Malformed session or selection"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn attach(
    State(app): State<AppState>,
    Json(request): Json<AttachRequest>,
) -> Result<Response> {
    let guard = AttachmentGuard {
        attachment: Id::new(),
        state: app.state.clone(),
    };
    let attachment = guard.attachment;
    let attached = app
        .state
        .ask(Attach {
            attachment,
            request,
        })
        .await?;
    let mut progress = Progress {
        attachment,
        replicas: app.replicas.clone(),
        live: app.live.clone(),
        replica: attached.replica.clone(),
        watching: StreamMap::new(),
    };
    progress.view();
    let updates = stream::unfold(Some(progress), |progress| async move {
        let mut progress = progress?;
        Some(match progress.next().await {
            Ok(event) => (event, Some(progress)),
            Err(reason) => (SseEvent::Ended(Ended { reason }), None),
        })
    });
    // Fused because the compression layer polls the body again after it ends,
    // and `unfold` panics when polled past its end.
    let events = stream::once(ready(SseEvent::Attached(attached)))
        .chain(updates)
        .fuse()
        .map(move |event| {
            let _ = &guard;
            Event::default().json_data(event)
        });
    Ok(Sse::new(events)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

/// What one attach stream has sent, and the screens it watches.
struct Progress {
    attachment: IdOf<Attachment>,
    replicas: watch::Receiver<Arc<Replica>>,
    /// Holds every pane of a replica the stream has, because the state actor
    /// publishes it first.
    live: watch::Receiver<LivePanes>,
    /// The newest replica sent, starting with the one in `Attached`.
    replica: Arc<Replica>,
    /// The screens of the viewed tab's panes in `replica`.
    watching: StreamMap<IdOf<Pane>, WatchStream<Arc<Screen>>>,
}

impl Progress {
    /// Waits for a newer replica or a watched screen. Replicas no newer than
    /// the one sent are skipped. The first newer replica without this
    /// attachment means its session was removed; a closed channel means
    /// shutdown.
    async fn next(&mut self) -> std::result::Result<SseEvent, EndReason> {
        loop {
            tokio::select! {
                changed = self.replicas.changed() => {
                    changed.map_err(|_| EndReason::ServerShutdown)?;
                    let replica = self.replicas.borrow_and_update().clone();
                    if replica.revision > self.replica.revision {
                        if !replica.viewers.contains_key(&self.attachment) {
                            return Err(EndReason::SessionRemoved);
                        }
                        self.replica = replica.clone();
                        self.view();
                        return Ok(SseEvent::State(replica));
                    }
                }
                Some((pane, screen)) = self.watching.next() => {
                    return Ok(SseEvent::Screen(PaneScreen { pane, screen }));
                }
            }
        }
    }

    /// Watch exactly the viewed tab's panes. A new watch yields its current
    /// screen first, so a quiet pane shows up at once.
    fn view(&mut self) {
        let sessions = &self.replica.sessions;
        let panes: Vec<IdOf<Pane>> = self
            .replica
            .viewers
            .get(&self.attachment)
            .and_then(|record| tree::viewed_tab(sessions, record.selection))
            .and_then(|tab| tree::tab(sessions, tab).ok())
            .map(|tab| tab.panes.keys().copied().collect())
            .unwrap_or_default();
        let gone: Vec<IdOf<Pane>> = self
            .watching
            .keys()
            .filter(|pane| !panes.contains(pane))
            .copied()
            .collect();
        for pane in gone {
            self.watching.remove(&pane);
        }
        let live = self.live.borrow();
        for pane in panes {
            if !self.watching.contains_key(&pane)
                && let Some(handle) = live.get(&pane)
            {
                self.watching
                    .insert(pane, WatchStream::new(handle.screen.clone()));
            }
        }
    }
}

/// Requires and parses `X-Ship-Attachment-Id`; missing or malformed is 400.
/// Whether the ID is active is the state actor's 404, not the extractor's.
pub(crate) struct AttachmentHeader(pub IdOf<Attachment>);

impl<S: Send + Sync> FromRequestParts<S> for AttachmentHeader {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self> {
        let value = parts
            .headers
            .get(ATTACHMENT_HEADER)
            .ok_or_else(|| err!(Validation, "missing {} header", ATTACHMENT_HEADER))?;
        let value = value.to_str().map_err(
            |error| err!(Validation, "malformed {} header", ATTACHMENT_HEADER, @external: error),
        )?;
        value.parse().map(Self)
    }
}

#[utoipa::path(
    put,
    path = "/api/v0/attach/view",
    operation_id = "view",
    params(("x-ship-attachment-id" = String, Header,
        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a...")),
    request_body = ViewInput,
    responses(
        (status = 200, description = "The attachment's viewing record as stored. A selection \
            no longer in the tree leaves it unchanged.", body = ViewingRecord),
        (status = 400, description = "Missing or malformed attachment header", body = AppError),
        (status = 404, description = "Attachment not found", body = AppError),
        (status = 422, description = "Malformed selection or size"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn view(
    State(app): State<AppState>,
    AttachmentHeader(attachment): AttachmentHeader,
    Json(view): Json<ViewInput>,
) -> Result<Response> {
    let record = app.state.ask(SetView { attachment, view }).await?;
    Ok(Json(record).into_response())
}

#[utoipa::path(
    put,
    path = "/api/v0/attach/selection",
    operation_id = "select",
    params(("x-ship-attachment-id" = String, Header,
        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a...")),
    request_body = SelectRequest,
    responses(
        (status = 200, description = "The attachment's updated viewing record", body = ViewingRecord),
        (status = 400, description = "Missing or malformed attachment header", body = AppError),
        (status = 404, description = "Attachment or selection not found", body = AppError),
        (status = 422, description = "Selection outside the attached session, or malformed selection", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn select(
    State(app): State<AppState>,
    AttachmentHeader(attachment): AttachmentHeader,
    Json(body): Json<SelectRequest>,
) -> Result<Response> {
    let record = app
        .state
        .ask(Select {
            attachment,
            selection: body.selection,
        })
        .await?;
    Ok(Json(record).into_response())
}

#[utoipa::path(
    put,
    path = "/api/v0/attach/session",
    operation_id = "switch_session",
    params(("x-ship-attachment-id" = String, Header,
        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a...")),
    request_body = SwitchSessionRequest,
    responses(
        (status = 200, description = "The attachment's updated viewing record, at the new session's root", body = ViewingRecord),
        (status = 400, description = "Missing or malformed attachment header", body = AppError),
        (status = 404, description = "Attachment or session not found", body = AppError),
        (status = 422, description = "Malformed session ID"),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn switch_session(
    State(app): State<AppState>,
    AttachmentHeader(attachment): AttachmentHeader,
    Json(body): Json<SwitchSessionRequest>,
) -> Result<Response> {
    let record = app
        .state
        .ask(SwitchSession {
            attachment,
            session: body.session,
        })
        .await?;
    Ok(Json(record).into_response())
}
