use std::{future::ready, time::Duration};

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
use ship_core::{AppError, Result, err, id::*, protocol::*};

use crate::{
    AppState,
    state::{Attach, Detach, Select, ServerState, SwitchSession},
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
            let _ = state.tell(Detach(attachment)).await;
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
                `attached` first, then `state` for each newer replica, then `ended` with \
                the reason when the attachment's session is removed or the server shuts down."),
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
    let seed = attached.replica.revision;
    // Replicas at or below the seed are already in it, or older. The first
    // newer replica without this attachment means its session was removed; a
    // closed channel means shutdown. Either ends the stream after `Ended`.
    let updates = stream::unfold(Some(app.replicas.clone()), move |replicas| async move {
        let mut replicas = replicas?;
        loop {
            if replicas.changed().await.is_err() {
                return Some((
                    SseEvent::Ended {
                        reason: EndReason::ServerShutdown,
                    },
                    None,
                ));
            }
            let replica = replicas.borrow_and_update().clone();
            if replica.revision <= seed {
                continue;
            }
            if !replica.viewers.contains_key(&attachment) {
                return Some((
                    SseEvent::Ended {
                        reason: EndReason::SessionRemoved,
                    },
                    None,
                ));
            }
            return Some((SseEvent::State(replica), Some(replicas)));
        }
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
