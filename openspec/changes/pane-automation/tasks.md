# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-19, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `ship-core` checks with and without `clap`;
- `openspec validate pane-automation --type change --strict`.

Use owned temporary servers on ports 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Don't touch existing user work, including the server on 43179, and clean up only what the exercise created. Scripted reporters are temporary probes and aren't committed. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. Observer attachments

- [x] 1.1 Make the attach area optional:
  - `AttachRequest.area` and `ViewingRecord.area` become `Option<Size>`, and `PROTOCOL_VERSION` becomes 14 (gate G3).
  - `tab_sizes` skips records without an area, and `SetView` stores `Some(area)`.
  - The full-screen client passes `Some` in `ui/mod.rs` and `ui/observer.rs`.
- [x] 1.2 Verify:
  - `POST /api/v0/attach` with `{}` gets `attached` and then `state` events and no `screen` events, and `ship view list` shows its record without an area.
  - With a full-screen client attached, attaching an observer with curl leaves the tab's published `geometry` unchanged and the client draws as before.
  - Reconnecting and resizing the full-screen client still work.
  - `ship server status` reports protocol 14.

## 2. Headerless input and `ship pane send`

- [x] 2.1 Make the input header optional:
  - `OptionalFromRequestParts` for `AttachmentHeader` in `attach.rs`.
  - `input` takes `Option<AttachmentHeader>`, `route` takes `Option<IdOf<Attachment>>` and rejects a headerless mouse frame, and the OpenAPI description is updated.
  - `send_stream` and `Client::input` take an optional attachment, and `send_stream` records the response's revision. The full-screen client passes `Some(attached.attachment)`.
- [x] 2.2 Add `ship pane send` without `--wait`:
  - `crokey` in `ship-core`'s manifest, and `pane::SendInput` (pane, text, keys) on `PaneCommand`.
  - `automation.rs` with `Input`, `Client::send` and `Client::write`, and the `Send` arm in `execute.rs`.
- [x] 2.3 Verify P2 against `cat -v` and a shell, read back with `ship pane read`:
  - Text then `--key esc --key enter` arrive in that order.
  - Multi-line stdin with `-` arrives as one paste and a single Enter.
  - `--key ctrl-c` interrupts `sleep`, and a pane scrolled into history returns to the bottom.
  - A headerless curl input with a mouse frame gets 422, and a malformed header still gets 400.
  - In the full-screen client, typing, pasting and a mouse selection's copy to its own clipboard still work, and a second attached client doesn't receive that copy.
  - `send` with neither text nor keys exits 2, and `--key ctl-c` exits 2 with nothing arriving.
  - An unknown or malformed ID fails naming the pane, and a pane whose program exited accepts the input silently.
  - `ship view list` shows no attachment from `send`.
- [x] 2.4 Gate G1 (U-1), scoped by Cyan to Pi with `openai-codex/gpt-6-luna`: send a prompt with `--key enter` and record in the deviation log whether the Enter was taken. Passed 3/3 trials on macOS; other agents and Linux remain unverified.

## 3. `ship pane wait` and `send --wait`

- [ ] 3.1 Gate G2 (U-2), with Cyan's OK, before writing this slice: with slice 2's `send` and a `ship pane get` loop, record what each reporting agent publishes around a prompt. A `working` later than 5 seconds, or an `idle` re-reported first, reopens FR-018 and FR-019.
- [ ] 3.2 Add the waits:
  - `pane::Wait`, `WaitState`, and `--wait` and `--status` on `SendInput`.
  - `Condition`, `Client::wait`, `Client::replicas`, `start` and `judge` in `automation.rs`, and `send` with a condition calling `wait`.
  - The `Wait` arm and the `--wait` path of the `Send` arm in `execute.rs`.
- [ ] 3.3 Verify P1, P3 and P4 against a scripted reporter, a shell script in a pane that reads a line, reports OSC 7501 `working`, sleeps, then reports `idle`, `blocked`, `done` or `error`, or exits:
  - SC-003: 50 of 50 `send --wait` runs return against a 50 ms `working`.
  - SC-004: each P4 failure prints its own message, and a reporter that never reports fails within 6 seconds.
  - SC-005: `timeout 2 ship pane wait` on a `working` pane exits 124 and leaves no observer in `ship view list`.
  - SC-002: switching tabs and panes in a full-screen client throughout changes nothing.
  - `--status done,exited` returns on either; `wait` on a condition that already holds returns at once.
  - `--status` on `send` without `--wait` exits 2.
  - `send --wait` on a pane already `working` returns when that turn ends.

## 4. Docs, specs and size

- [ ] 4.1 Document the user-visible behavior:
  - `docs/guide.md`: "Driving panes from scripts" after "Scrolling and reading output", covering everything FR-023 lists, and an Upgrading line for protocol 14.
  - `docs/product-brief.md`: milestone 4 as FR-022 lists.
  - `README.md`: line 16, "Not built yet" and "What works today".
  - The deltas under `specs/`: bring them in line with anything the deviation log changed.
- [ ] 4.2 Verify:
  - `openspec validate pane-automation --type change --strict`, every link resolves, and the guide's walkthrough runs as written, including `timeout(1)` and the macOS `gtimeout` note.
  - SC-001 by hand from inside a Ship pane with a real agent, with Cyan's OK.
  - Implementation Rust and test lines reported separately against the count before this change.
- [ ] 4.3 When the PR opens, file a tracking issue for exit followed by close (architecture "Risks"), and issues for anything else left open.
