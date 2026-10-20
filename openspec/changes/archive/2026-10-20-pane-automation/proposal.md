# Proposal

This is the planning artifact for a locked plan. Cyan locked the three papers in `design/` on 2026-10-19; the product and architecture papers were amended the same day during later stages. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

The product brief's milestone 4 asks that an agent in one pane can drive another: start a sibling, give it work, wait for the result and read it. `ship pane create`, `ship tab list`, `ship pane get` and `ship pane read` already cover starting, listing and reading. Two pieces are missing: sending input to a pane, and waiting for its program status to change.

## What Changes

- **`ship pane send --pane ID [TEXT] [--key KEY]...`** writes TEXT as one paste, then each key in order. `-` reads the text from stdin. Key names are the keymap config's (`enter`, `ctrl-c`), and an invalid one is a usage error that sends nothing. Without `--wait` it prints nothing and doesn't attach.
- **`ship pane wait --pane ID [--status STATE,...]`** waits until the pane's root status record leaves `working`, or matches one of the listed states (`idle`, `working`, `done`, `blocked`, `error`, `exited`), then prints the pane as `ship pane get` would. It returns at once if the condition already holds, and has no timeout of its own.
- **`send --wait [--status ...]`** refuses a pane that has no program status, is `blocked`, or has exited, before writing anything. After writing, it waits for the program to report `working` or `blocked`, then applies the condition. It fails with "stalled" if neither is seen within 5 seconds of the write.
- **Targeting** is by `--pane`, else `SHIP_PANE_ID`, never by an attachment's selection.
- **Observers.** An attachment can attach without an area. It sizes no tab, and with no selection it gets state replicas and no screens. Waits use one.
- **Headerless input.** `POST /api/v0/attach/input` accepts requests without `x-ship-attachment-id`. Keys and pastes route as before; a mouse frame gets 422.
- **Protocol 13 to 14,** because `ViewingRecord.area` becomes optional and a protocol-13 client couldn't decode an observer's record.

Not in this change (the product paper lists them all):
- Agent detection, agent names or an `agent` command group.
- Waiting for text in a pane's output.
- A `--timeout` flag; callers use `timeout(1)`.
- An event stream for CLI callers.
- Waiting on non-root records or on several panes.
- Detecting an agent that exited to a shell and left a stale record behind.

## Capabilities

### New Capabilities

- `pane-automation`:
  - `ship pane send` and `ship pane wait`, their targeting, condition, exit and lifetime rules.
  - `send --wait`'s checks before writing, its pickup rule and the 5-second stall.
  - The input stream without an attachment header.

### Modified Capabilities

- `session-observation`: observer attachments without an area; view records omit `area` when there is none.
- `health-exchange`: protocol version 14.

## Impact

- `ship-core`:
  - `pane::SendInput`, `pane::Wait` and `WaitState` in `command.rs`, all CLI-only.
  - `AttachRequest.area` and `ViewingRecord.area` become `Option<Size>`, and `PROTOCOL_VERSION` becomes 14.
  - `crokey` becomes a dependency.
- `ship-server`:
  - `tab_sizes` skips records without an area.
  - `AttachmentHeader` implements `OptionalFromRequestParts`, and the input route takes an optional header.
- `ship-client`:
  - A new `automation` module with the input, writer, condition and wait loop.
  - `execute.rs` gains two arms.
  - `Client::input` takes an optional attachment, and the input response's revision is recorded.
  - The full-screen client passes `Some` for its area and attachment.
- `ship`: none.
- Docs:
  - A guide section on driving panes from scripts, and an Upgrading line for protocol 14.
  - The product brief's milestone 4.
  - The README's "What works today" and "Not built yet".
- Estimated at about 220 lines of implementation Rust. Measured counts go in [tasks.md](tasks.md).
