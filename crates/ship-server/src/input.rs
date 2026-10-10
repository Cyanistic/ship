//! The input stream: keys and pastes from one attachment, routed to panes.

use std::io;

use axum::{
    Json,
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::{StreamExt, TryStreamExt};
use ship_core::{AppError, Result, err, protocol::*};
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};

use crate::{
    AppState,
    attach::AttachmentHeader,
    pane::{LivePanes, PaneCommand},
    state::CheckAttachment,
};

#[utoipa::path(
    post,
    path = "/api/v0/attach/input",
    operation_id = "input",
    params(("x-ship-attachment-id" = String, Header,
        description = "Attachment ID from the stream's `attached` event, e.g. attachment:3f2a...")),
    request_body(content = InputFrame, content_type = "application/x-ndjson",
        description = "One `InputFrame` per line, each at most 64 KiB, read as they arrive. \
            Frames for panes that aren't running are ignored."),
    responses(
        (status = 204, description = "The body ended, or the attachment or server did"),
        (status = 400, description = "Missing or malformed attachment header", body = AppError),
        (status = 404, description = "Attachment not found; nothing was read", body = AppError),
        (status = 422, description = "Malformed or oversized line; reading stopped there", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn input(
    State(app): State<AppState>,
    AttachmentHeader(attachment): AttachmentHeader,
    body: Body,
) -> Result<Response> {
    let revision = app.state.ask(CheckAttachment(attachment)).await?;
    let mut replicas = app.replicas.clone();
    let body = StreamReader::new(body.into_data_stream().map_err(io::Error::other));
    let mut lines = FramedRead::new(body, LinesCodec::new_with_max_length(INPUT_LINE_MAX));
    loop {
        tokio::select! {
            line = lines.next() => {
                let Some(line) = line else { break };
                match line
                    .map_err(|error| err!(Validation, "unreadable input line", @external: error))
                    .and_then(|line| frame(&line))
                {
                    Ok(Some(frame)) => route(&app.live.borrow(), frame),
                    Ok(None) => {}
                    Err(error) => {
                        return Ok((StatusCode::UNPROCESSABLE_ENTITY, Json(error)).into_response());
                    }
                }
            }
            // Replicas older than the check may still be on their way and
            // lack the attachment, so only newer ones can end the stream.
            changed = replicas.changed() => {
                if changed.is_err() {
                    break;
                }
                let replica = replicas.borrow_and_update();
                if replica.revision >= revision && !replica.viewers.contains_key(&attachment) {
                    break;
                }
            }
        }
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// A blank line is skipped.
fn frame(line: &str) -> Result<Option<InputFrame>> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    Json::<InputFrame>::from_bytes(line.as_bytes())
        .map(|Json(frame)| Some(frame))
        .map_err(|error| err!(Validation, "malformed input frame", @external: error))
}

fn route(live: &LivePanes, frame: InputFrame) {
    let (pane, command) = match frame {
        InputFrame::Key(KeyInput { pane, key }) => (pane, PaneCommand::Key(key)),
        InputFrame::Paste(PasteInput { pane, text }) => (pane, PaneCommand::Paste(text)),
    };
    if let Some(handle) = live.get(&pane) {
        handle.commands.send(command).ok();
    }
}
