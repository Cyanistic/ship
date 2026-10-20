# Proposal

This is the planning artifact for a locked plan. Cyan locked the three papers in `design/` on 2026-10-20. The product paper was revised the same day to drop a `data.sent` field and to respell error keys and kinds. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

Successful `ship` commands print JSON, but a failure prints a line of prose such as `ship: the input was sent: pane pane:5157… stalled: ...`. Scripts and agents have to parse JSON on success and split sentences on failure, and the prose drops the error's category and blurs where one cause ends and the next begins. Issue #37.

## What Changes

- **JSON error line.** A failed command prints its error as one JSON line on stderr, the same object the HTTP API returns as an error body, and exits 1. The line is the last thing on stderr, and stdout stays empty. There's no flag and no terminal detection.
- **BREAKING: error spelling.** The cause key becomes `causedBy` (was `caused_by`), and error kinds become snake_case (`not_found`, was `NotFound`). This applies to HTTP error bodies too.
- **`ship config check`** prints its findings as text and exits 1 when there are any, without the extra `ship: invalid config` line.
- **Unchanged:** clap's usage errors (text, exit 2), `--help`, `--version`, and the `send --wait` messages "nothing was sent", "the input was sent" and "the input may have been sent", which become the outermost `message`.

Not in this change (the product paper lists them all):
- Errors in the full-screen client's status line.
- JSON log lines.
- New exit codes.
- A separate field saying whether `send`'s input went out.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `health-exchange`: new requirements for the error body's shape and spelling, and for the CLI error line.
- `pane-automation`: automation errors use the JSON error line.
- `config-file`: `ship config check` exits 1 after its findings with no further error line.

## Impact

- `ship-core`: two serde attributes in `error.rs`.
- `ship`: `main` prints errors as JSON, `run` and `dispatch` return an exit code, and `check_config` returns one.
- `ship-server` and `ship-client`: no code changes. They pick up the new spelling from `ship-core`.
- Docs: a guide section on reading errors. The README doesn't change.
- Estimated at about 20 lines of implementation Rust. Measured counts go in [tasks.md](tasks.md).
