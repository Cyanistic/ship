# Pane automation: architecture

Status: Locked on 2026-10-19, approved by Cyan after Plannotator review ("LGTM!"). Written the same day from a design conversation with Cyan. Implements the locked `product.md` beside it, as amended on 2026-10-19. Locking doesn't authorize implementation.

Amended on 2026-10-19 during program design at Cyan's direction: the input stream's attachment header becomes optional, so plain `send` writes without attaching and `send --wait` attaches only to read replicas. Decision 4, shape A, the structure, the observer, precheck and writer boxes, and the commit-cost risk changed. Cyan approved the amendment without a further review ("i don't have to review the architecture one!").

`send` and `wait` are client-side commands. To wait, the CLI attaches as an observer, an attachment with no area and no selection, so it gets each published replica but no screens and has no effect on any tab's size. It writes input through the existing input stream, without an attachment, and decides when a wait is over by reading replicas, just as the full-screen client reads them. The published revision is the source of truth: a status that never reaches a revision didn't happen. The server gains an optional attach area and an optional attachment header on the input stream. It gets no new route, counter or waiter registry.

## Context and constraints

- **Status reaches clients only through revisions.** The pane task applies OSC 7501 reports one by one, but `publish` (`crates/ship-server/src/pane.rs:578`) sends one `PaneChange::Program` with the final record set at most once per `FRAME` (4 ms, `pane.rs:47`). `ServerState` commits it to the tree, and every commit publishes a whole `Replica` on the `app.replicas` watch. A `working` that turns into something else within one frame never reaches a revision. Under the amended FR-018 that is accepted: it didn't happen.
- **The attach stream starts gapless and then sends the latest revision.** `attach` (`crates/ship-server/src/attach.rs`) returns the replica committed with the new attachment inside `Attached`, then `Progress::next` sends each newer replica it reads with `borrow_and_update`. A subscriber that falls more than one commit behind skips the commits in between. For status, that takes two commits closer together than the stream polls.
- **Attachments without a selection already act as observers.** `tab_sizes` (`crates/ship-server/src/state.rs:252`) skips viewing records with no selection, and `Progress::view` watches screens only for the selected tab. An attachment that never selects anything is already left out of geometry and receives no `screen` events. `AttachRequest.area` and `ViewingRecord.area` are still required fields, though.
- **The input stream already writes keys and pastes in order.** `POST /api/v0/attach/input` (`crates/ship-server/src/input.rs`) reads NDJSON `InputFrame`s and routes each one to the pane's unbounded command channel as `PaneCommand::Paste` or `PaneCommand::Key`. One channel per pane keeps the order. Frames for removed panes are dropped, and an exited pane ignores keys and pastes (`PaneTask::command`). The request requires an attachment header, which only mouse frames use: the pane task records the pointer's attachment so a mouse selection's copy goes to that client's clipboard (`pane.rs:550`, `pane.rs:594`). Keys and pastes never read it. The header also ends the request when its attachment ends, a backstop for the full-screen client, which drops its own input request on disconnect (`ui/mod.rs:260`).
- **Every response carries the revision.** The `stamp` middleware (`crates/ship-server/src/app.rs:37`) sets `x-ship-revision` after the handler runs, so the input stream's response carries a revision at least as new as the one current when its last frame was routed.
- **The CLI's request timeout.** Ordinary requests time out after 2 seconds (`crates/ship-client/src/lib.rs:33`). Opening the attach stream is covered by that timeout, but reading it is not, so a wait can last as long as it needs.
- **Exit and records.** The `PaneChange::Exited` commit sets `PaneStatus::Exited` and clears `program` in the same revision (`state.rs:973`). Panes stay in the tree after exit today. Close-on-exit isn't built.
- **Key names.** The keymap config parses keys with crokey (`crates/ship-client/src/keymap.rs`). `KeyInput.key` carries a crossterm `KeyEvent`.
- **Compactness.** About 20k lines of implementation Rust in total. The product estimate for this change is 200 to 350 lines.

## Candidate shapes

### A: Observer attachment, waiting in the CLI (chosen)

For a wait, the CLI attaches with no area and no selection, checks the snapshot in `Attached`, writes through `/attach/input` without an attachment header, notes the revision stamped on that response, then reads `state` events until the pane matches, exits or disappears. Plain `send` checks the pane with `GET /panes/{id}` and writes, without attaching. The server changes by making the attach area and the input stream's attachment header optional. Cost: two extra commits (attach and detach) per waiting command, each publishing a replica to every client. Failure mode: two commits back to back can be seen as one, which matters only once close-on-exit removes a pane in the commit after its exit.

```text
ship pane send --wait
  ├─ POST /attach (no area) ──────────▶ ServerState  commit viewer, reply snapshot
  │     ◀── attached{replica R0}
  ├─ check R0: pane exists, root not blocked
  ├─ POST /attach/input, no header ────▶ pane task   write paste, keys in order
  │     ◀── 204, x-ship-revision = S
  └─ read state events with revision > S
        pane gone / exited / gate / condition → print pane or error

ship pane send (no --wait)
  ├─ GET /panes/P ────────────────────▶ ServerState  pane exists
  └─ POST /attach/input, no header ────▶ pane task   write paste, keys in order
```

### B: Waiters registered in the state actor

`send` and `wait` are server routes. The pane task counts `working` and `blocked` reports as they arrive and attaches the count to `PaneChange::Program`. `ServerState` keeps the counts and a waiter registry in private maps and checks a pane's waiters on every commit for that pane. It catches a `working` shorter than a frame and sees an exit before any removal. Cost: a counter, a registry and two routes inside the server, for precision the amended product paper no longer wants. Failure mode: the registry leaks waiters if the closed-sender cleanup is missed.

```text
pane task ──Program{records, busy}──▶ ServerState ── on commit: check waiters for pane
CLI ──POST /panes/P/send|wait──▶ route ──register(cond, baseline)──▶ waiters
                                  ◀── matched pane / exited / closed
```

### C: Polling `pane get`

The CLI writes through a new `POST /panes/{id}/send` route, then calls `GET /panes/{id}` every 50 ms or so until the condition holds. Cost: a new route, and up to one poll interval of extra latency on every wait. Failure mode: polling skips revisions shorter than the interval, so a quick `working` is missed more often than on the stream, and many waiting CLIs each keep the actor busy.

```text
CLI ──POST /panes/P/send──▶ pane task
CLI ──GET /panes/P──▶ ServerState    (repeat every 50 ms until matched)
```

## Decision

A. What mattered most was that a wait sees exactly what every other client sees. A reads the same published revisions as the full-screen client and adds almost nothing to the server. B builds precision the product no longer asks for, and C is slower and misses more.

## Structure

```text
CLI (ship-client)
  send / wait command
    target       --pane, else SHIP_PANE_ID, else error
    keys         crokey names → crossterm KeyEvent, all parsed before any write
    observer     send --wait and wait: POST /attach {selection: none, area: none}; detaches on drop
    precheck     from the Attached snapshot, or GET /panes/P for plain send: pane exists;
                 for send --wait, root present and not blocked
    writer       POST /attach/input without a header: one paste frame, then key frames; keeps x-ship-revision
    waiter       reads state events; gate (send --wait only), condition, exit, gone, 5 s stall timer
    printer      the matched pane as JSON, as `pane get` prints it

Server (ship-server)
  attach         area optional on AttachRequest and ViewingRecord
  tab_sizes      also skips records without an area
  input stream   attachment header optional; without it, keys and pastes only
```

What each box owns:

- **Target**: resolving the pane from `--pane` or `SHIP_PANE_ID` through the existing `PaneTarget`, and failing when neither is set (FR-001). It never reads an attachment's selection.
- **Keys**: turning `--key` values into `KeyEvent`s with the parser the keymap uses, and failing before anything is written when one doesn't parse (FR-005).
- **Observer**: one attachment per waiting command, opened before anything else. Plain `send` has none. Dropping the stream ends the attachment through the existing `AttachmentGuard`, so a killed CLI leaves nothing behind (FR-014).
- **Precheck**: everything that has to fail before a write, read from the `Attached` replica, or for plain `send` from `GET /panes/{id}`. The pane must exist (FR-002). For `send --wait`, the root record must be present and not `blocked` (FR-017). For `wait`, a missing root record without `exited` in the condition fails at once (FR-012), and a condition that already holds returns at once (FR-009). It also records whether the root was `working` at that point, which opens the gate in FR-018.
- **Writer**: the text as one `Paste` frame, then one `Key` frame per key in order (FR-004), on a single input request without an attachment header. The pane task's existing handling returns a scrolled pane to the bottom (FR-006) and drops input to an exited program (FR-007). The response's `x-ship-revision` is the boundary between before and after the write.
- **Waiter**: a loop over `state` events. For each replica: the pane gone means "pane closed" (FR-002). `PaneStatus::Exited` matches when `exited` is in the condition and otherwise fails with the exit status (FR-010, FR-011). For `send --wait`, the gate opens on the first replica newer than the boundary whose root is `working` or `blocked`, or is already open from the precheck (FR-018). Once the gate is open, a root that matches the condition ends the wait (FR-008). The 5-second stall timer runs only while the gate is closed (FR-019). An `ended` event means the server stopped.
- **Printer**: the pane from the replica that matched, in the same JSON as `ship pane get` (FR-013).
- **Attach (server)**: the area becomes optional. A record without one is left out of `tab_sizes`, the same as a record without a selection.
- **Input stream (server)**: the attachment header becomes optional. With it, nothing changes: an unknown attachment gets 404, the request ends with the attachment, and mouse frames are allowed. Without it, keys and pastes are routed as before and a mouse frame is rejected like a malformed line, since a mouse selection's copy needs an attachment to go to. Nothing else on the server changes.

## Key decisions

1. **The published revision is the source of truth.** A wait sees exactly what any client sees, and a status that never reaches a revision didn't happen (amended FR-018). Rejected: counting reports in the pane task with a baseline (shape B), and Herdr's event replay with `accept_transient_status`. Both exist to catch states shorter than a frame, which no normal workflow needs.
2. **The waiting happens in the CLI, not the server.** The CLI reads replicas the way the full-screen client already does, so the server keeps no per-wait state, and a killed CLI cleans up through the attachment guard. Rejected: a waiter registry in `ServerState` (shape B) and polling `pane get` (shape C).
3. **Observers are attachments with no area and no selection.** Ship already leaves selectionless records out of geometry and screens. Making the area optional states that in the protocol instead of having the CLI send a made-up size. Rejected: a dummy area such as 0×0, which works today but breaks as soon as something gives the observer a selection, a client-kind enum on the record (CLI or full-screen), which names who the client is when the server only cares whether it has an area, and a separate status-only stream endpoint, which would duplicate `attach`.
4. **Input goes through the existing input stream, without an attachment** (amended). `/attach/input` already delivers pastes and keys in order to the pane's command channel, which is exactly FR-004. Only mouse frames need an attachment, so the header becomes optional and the CLI sends none; plain `send` then costs no commits. Rejected: a new `POST /panes/{id}/send` route, a second way to write the same frames; attaching just to get an ID for the header, which costs two commits per `send`; moving the attachment into `MouseInput` and dropping the header, which loses the full-screen client's lifetime backstop and the `attachment_id` in request logs for no gain.
5. **The response's revision marks the write.** `x-ship-revision` on the input response is read after the frames are routed, so a replica newer than it was committed after the input was queued. Rejected: treating everything after the `Attached` snapshot as after the write, which would count a status published between attaching and writing.
6. **Errors are decided from the snapshot before writing.** The `Attached` replica holds the tree at a known revision, so the pane-exists, blocked and no-record checks need no extra requests. The input stream drops frames for unknown panes silently, so the CLI has to make those checks itself (FR-002).
7. **`send` and `wait` are CLI-only.** They go in `ship-core`'s `pane::PaneCommand` with `#[serde(skip)]`, like `get` and `read`, so the keymap config rejects them. Binding text to keys is a separate feature.

## Risks and unknowns

- **Exit followed by close.** Once close-on-exit exists, the exit commit and the removal commit come back to back, so the stream can skip from running straight to gone and the waiter loses the exit status FR-010 requires. Today panes stay after exit, so this can't happen yet. The close-on-exit change has to handle it, for example by keeping the exit status in the removal or delaying removal by one revision. A tracking issue is filed when this change merges.
- **Two commits per waiting command.** Each `send --wait` or `wait` commits an attach and a detach (plain `send` commits nothing), and each commit publishes a full replica to every client. Compressed replicas measured small in earlier work, but a script that waits in a tight loop multiplies that. Not profiled yet, and worth checking if wait-heavy scripts feel slow.
- **Observers in `viewers`.** Observer records appear in each replica's `viewers` map. No client reads other attachments' records today (`ui/observer.rs` reads only its own). Anything that later counts viewers has to skip records without an area.
- **Already working at send.** By the amended FR-018, `send --wait` on a pane already `working` returns when that turn ends, not when the new input is handled. This matches Herdr and goes in the guide.
- **U-1 and U-2 stay open.** A gap between the paste and Enter (U-1) would sit in the writer, between frames. Agents that report `working` late or not at all (U-2) would hit the 5-second stall. Neither changes the shape.
- **Platforms.** Nothing here is platform-specific. SC-001 still needs a run on Linux and macOS.
