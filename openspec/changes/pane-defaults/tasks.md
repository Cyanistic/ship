# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-21, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `openspec validate pane-defaults --type change --strict`.

Use owned temporary servers on ports 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Don't touch existing user work, including the server on 43179. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 0. Branch

- [x] 0.1 Rebase `pane-defaults` onto `main` (it sits at `7ac103e`, 14 commits behind), keeping the uncommitted `openspec/changes/pane-defaults/` files.

## 1. Pane fields and the defaults table

- [x] 1.1 Add `ExitBehavior` and the `exit_behavior` and `scrollback_bytes` fields on `Pane` in `crates/ship-core/src/model.rs`, with serde defaults for old saved files.
- [x] 1.2 Add `PaneDefaults` and `builtin_scrollback_bytes` to `crates/ship-core/src/config.rs`, and replace `[server]` in `defaults.toml` with `[server.defaults.pane]`.
- [x] 1.3 Restructure `ServerSettings` in `crates/ship-server/src/settings.rs` around `defaults.pane`, resolving a relative `shell` from the file's folder.
- [x] 1.4 Make `PaneInput`'s three settings optional, bump `PROTOCOL_VERSION` to 15, and add `exit_behavior`, `scrollback_bytes` and the optional-value `right_click_passthrough` to `pane::Create` in `command.rs`. Pass them through in `execute.rs`.
- [x] 1.5 In `state.rs`, resolve absent settings from `pane_env.settings()` once in `start_pane`, pass shell and history size through `Launch`, and give restored panes their saved values. Remove the settings read from `pane::spawn`.
- [x] 1.6 Verify slice 1's checks in `design/program.md`: views show both fields, a saved table applies to new panes only, create flags and key-binding fields override it, a relative shell resolves, `config check` names a leftover `[server] shell`, a broken file gives built-in defaults and a log line, settings survive a restart, an old saved file restores with built-in defaults, and a protocol-14 client is refused.

## 2. The every feed

- [ ] 2.1 Add `crates/ship-server/src/replicas.rs` with `Replicas`, `ReplicaSink` and `RING = 256`, and wire it into `AppState`, `lib.rs`, the saver and `input.rs`.
- [ ] 2.2 Add `FeedMode` and `AttachRequest.feed`, and `EndReason::Lost`, in `protocol.rs`. In `attach.rs`, build the `Feed` before the attach commit and handle `Lagged` as `Lost`.
- [ ] 2.3 Make the client's `observe()` ask for `every` and `next()` report `Lost` as "fell behind the server while waiting on pane P". Add the `Lost` arm to the UI, handled as a dropped stream.
- [ ] 2.4 Verify slice 2's checks: consecutive revisions on `every`, coalescing on the default feed, `lost` under a temporary `RING = 2` probe (then revert), and unchanged waits and UI.

## 3. Close on exit

- [ ] 3.1 Add `ExitBehavior::closes`, and in the `PaneEvent` handler commit the exit and then, when it closes, the close as `Remove<Pane>` does.
- [ ] 3.2 Add `exit_behavior` to `SetPane` and `pane::Set`, applied in the `Set<Pane>` handler and passed through in `execute.rs`.
- [ ] 3.3 Verify slice 3's checks: `exit` closes a split and a lone pane, `keep` stays and reads back, `wait --status exited` on a closing pane prints the status, a plain wait fails with the exit code, `keep_failed` for code 0, code 3 and a signal, and `set --exit-behavior` on running and exited panes from the CLI and a key binding.

## 4. History size on a live pane

- [ ] 4.1 Add `SessionCommand::ScrollbackBytes` and `SessionHandle::set_scrollback_bytes` to `vendor/ratatui-ghostty/src/session.rs`.
- [ ] 4.2 Add `PaneCommand::Scrollback`, and `scrollback_bytes` to `SetPane` and `pane::Set`. Send the command from the `Set<Pane>` handler when the value changes. Document raising and lowering in `ship pane set --help`.
- [ ] 4.3 Verify slice 4's checks: lowering drops the oldest lines at once, raising keeps history and lets it grow, and the help text says both.

## 5. Docs and specs

- [ ] 5.1 Update the guide in `docs/site/`, following `docs/agents/docs.md`. Each item goes on the page whose scope fits it:
  - `configuration.md`: rewrite "Server settings" around `[server.defaults.pane]`, covering the per-pane history size and what raising or lowering it with `ship pane set --scrollback-bytes` does.
  - `panes.md`: rewrite the intro's "its pane stays open" for exit behavior, with `close`, `keep` and `keep_failed`.
  - `scripting/reading-output.md`: P2's agent workflow, which creates a pane with `--exit-behavior keep`, reads it, then closes it.
  - `saved-tabs.md`: panes keep their exit behavior and history size across a restart.
- [ ] 5.2 Remove the "Not built yet" line about closing panes on exit from `README.md` and from the hand-written landing page `website/src/landing/body.html`. Move the landing page's config example `shell` from `[server]` to `[server.defaults.pane]`. The README no longer has a config example.
- [ ] 5.3 Check the delta specs under `specs/` against the real output from slices 1 to 4, and the guide's examples against captured output. `openspec validate pane-defaults --type change --strict` passes.

## 6. Size

- [ ] 6.1 Report the change's implementation Rust and test line counts separately.
