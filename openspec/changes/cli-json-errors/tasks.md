# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-20, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `openspec validate cli-json-errors --type change --strict`.

Use owned temporary servers on ports 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Don't touch existing user work, including the server on 43179. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. Error spelling

- [x] 1.1 Add `rename_all = "snake_case"` to `ErrorCode` and `rename_all = "camelCase"` to `InternalError` in `crates/ship-core/src/error.rs`.
- [x] 1.2 Verify:
  - `curl` for a missing pane returns a 404 body with `"type":{"kind":"not_found"}`.
  - A failure with a cause shows `causedBy`.
  - `ship pane get --pane <missing>` still shows the server's message, so the client reads the new spelling.

## 2. JSON error line and config check exit

- [ ] 2.1 In `crates/ship/src/main.rs`, make `run` and `dispatch` return `Result<ExitCode>`, with one `.map(|()| ExitCode::SUCCESS)` after the dispatch match. `main` prints `Err` as one `serde_json` line on stderr, falling back to `ship: {error}`, and exits 1.
- [ ] 2.2 Make `check_config` in `crates/ship/src/commands.rs` return `Ok(ExitCode::FAILURE)` after printing findings, and return the code from its dispatch arm early.
- [ ] 2.3 Verify:
  - `ship pane get --pane <missing>`: the last stderr line parses with `jq`, `.type.kind == "not_found"`, stdout is empty, and the exit status is 1.
  - `send --wait` on a blocked pane gives `.message == "nothing was sent"`. On a pane that never reports, it gives `.message == "the input was sent"` with the stall under `.causedBy`.
  - `ship pane read --pain x` prints clap's text and exits 2. `ship --help` and `ship --version` exit 0 as before.
  - `ship config check` on a broken file prints the findings, no JSON, and exits 1. On a valid file it prints nothing and exits 0.
  - With `SHIP_LOG=debug`, the JSON is still the last stderr line.

## 3. Guide

- [ ] 3.1 Add a short section to `docs/guide.md` on reading a failed command's error with `jq`, telling from a `send --wait` error whether the input went out, and exit statuses 1 and 2, using output captured in slice 2.
- [ ] 3.2 Verify: the guide's example matches the real output, and the delta specs still match the behavior from slice 2. `openspec validate cli-json-errors --type change --strict` passes.

## 4. Size

- [ ] 4.1 Report the change's implementation Rust and test line counts separately.
