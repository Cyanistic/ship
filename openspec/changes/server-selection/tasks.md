# Tasks

Checkpoints for implementing the locked plan, one slice at a time. The papers in `design/` were locked on 2026-10-13 and are the contract; `design/program.md` holds the file map, signatures and full checks for each slice. Complete and verify one slice, then stop for Cyan's review before the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds, and ship-core checks with and without `clap`. Use owned temporary servers and config files, don't restart existing user work, and clean up only what the exercise created. Checks run on macOS; Linux counts only when run there, and is otherwise reported unverified.

## 1. Bindings to one named action with a hint

- [ ] 1.1 Add `strum` with `derive` to the workspace, `ship-core` and `ship-client`, and `IntoStaticStr` with snake_case names on `TabCommand`, `PaneCommand`, `Direction` and the client action enums. Verify the build, and that each derived name matches the config's spelling of that action.
- [ ] 1.2 Derive `Binding { action, hint }` with `deny_unknown_fields`, add `Action::hint`, and remove `action_hint` and the hand-written deserializer. Complete hints in `normalized`, skip unbound chords in `Mode::hints`, and update `enters_mode` and the `queued` call site in `ui/mod.rs`. Verify with `ship config check` on temporary files: `{ action = { server.tab.create = {} } }`, the same with `hint = "new"`, and `"alt-q" = {}` pass. `[{ server.tab.create = {} }]`, `{ server.tab.create = {} }` and `{ action = { … }, hnt = "x" }` fail with line and column.
- [ ] 1.3 Rewrite `keymap/defaults.toml` in the new syntax, without `select`. Verify in a running client that every default key works as before, `"alt-q" = {}` sends `alt-q` to the shell, `alt-r` shows `h left  j down  k up  l right  esc normal`, and a binding with `hint = "fill"` shows `fill` in place of the derived word.

## 2. `ship view` reads and steers a client's selection

- [ ] 2.1 Add `SelectView` and protocol version 10 in `ship-core`, the `ListViews`, `GetView` and `Select` messages in the state actor, with `Select` unzooming a hidden pane's tab in the same commit, and the three `/api/v0/views` routes. Verify with `curl` against a temporary server: list, get, select a pane, a tab and `null`, and 404 for an unknown attachment or node with nothing changed.
- [ ] 2.2 Add the three `Client` methods and `ship view list | selected | select`, with `--attachment` from `SHIP_ATTACHMENT_ID` and `--none` (G2). Verify SC-003: with a client attached, `ship view list` shows it, and `ship view select` to a pane, a tab and `--none` each move it within a frame while `ship view selected` prints each. Neither `NODE` nor `--none` exits 2. Selecting a zoom-hidden pane unzooms the tab for a second client. `ship server status` reports protocol 10.

## 3. Creates select what they make

- [ ] 3.1 Add `select` and `attachment` to both create commands, `select` to both request bodies, and `select_created` inside the create commits. Verify SC-004: `ship pane create --tab T -- sleep 9` moves no client, and with `--select --attachment A` client A lands on the new pane while `ship view list` in a loop never shows it unselected. `--select` without an attachment fails before creating, and an unknown attachment creates nothing and leaves no shell running.
- [ ] 3.2 Fill the key scope's attachment in `execute`, set `select = true` on the four default creates, and remove the client's `creates` path. Verify that `alt-n`, `alt-shift-n`, `alt--` and `alt-|` still land on what they made, and that a binding without `select` leaves the selection alone.

## 4. The lane orders everything a client does

- [ ] 4.1 Write the throwaway latency proxy in `experiments/latency-proxy/` (G4), delaying each chunk 500 ms each way. Verify that `ship server status` through it takes about a second.
- [ ] 4.2 Add `ui/lane.rs`, `Local` and its turn, `Request` in place of `Pending`, `Keys::handle` without a pane, `Mouse::route` with `busy` (G1), and the lane's 404 rule (G3). Reduce the view PUT and `SetView` to area. Verify SC-001 through the proxy: ten runs each of `alt-n` + `echo hi` + Enter and `alt-|` + `alt-f`, all landing right.
- [ ] 4.3 Verify the order rules through the proxy. A click on another pane during a pending create ends there. A failed action shows its error and later keys still arrive. Killing the proxy mid-burst drops what wasn't sent and replays nothing. `alt-q` after typing detaches after the typing lands. A script's `ship view select` survives a terminal resize. Typing with a tab selected reaches its landing pane.
- [ ] 4.4 Verify SC-002 against a local server: typing, navigation, key repeat and clicks feel no slower in daily use.

## 5. Docs, spec sync and size report

- [ ] 5.1 Update `docs/guide.md` (binding syntax and hints, `--select`, `ship view`), `README.md`'s config example, `GLOSSARY.md` "Selection", the `AGENTS.md` "Client/server boundary" section, and an amendment line atop the archived keys-config product paper. Verify by running each documented command as written against a temporary server, and that every link resolves.
- [ ] 5.2 Bring the deltas under `specs/` in line with what was built, and run `openspec validate server-selection`. Verify that every deviation in the program paper's log is reflected in a delta or noted as not user-visible.
- [ ] 5.3 Bring `handoff.md` and the product paper's deferred issues to Cyan, and remove `experiments/latency-proxy/`. Verify that nothing is filed without Cyan's approval.
- [ ] 5.4 Report implementation Rust and test line counts separately against today's count and the 150-line estimate (SC-005).
