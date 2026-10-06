use std::{future::ready, time::Duration};

use axum::{
    Json,
    extract::State,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures_util::{StreamExt, stream};
use kameo::actor::ActorRef;
use ship_core::{AppError, Result, id::*, protocol::*};
use tokio_stream::wrappers::WatchStream;

use crate::{
    AppState,
    state::{Attach, Detach, ServerState},
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
                `attached` first, then `state` for each newer replica. Ends when the \
                attachment's session is removed."),
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
    // Replicas at or below the seed are already in it, or older; the first
    // newer replica without this attachment means its session was removed.
    let updates = WatchStream::new(app.replicas.clone())
        .filter(move |replica| ready(replica.revision > seed))
        .take_while(move |replica| ready(replica.viewers.contains_key(&attachment)))
        .map(SseEvent::State);
    let events = stream::once(ready(SseEvent::Attached(attached)))
        .chain(updates)
        .map(move |event| {
            let _ = &guard;
            Event::default().json_data(event)
        });
    Ok(Sse::new(events)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}
