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

- [x] 2.1 Add `crates/ship-server/src/replicas.rs` with `Replicas`, `ReplicaSink` and `RING = 256`, and wire it into `AppState`, `lib.rs`, the saver and `input.rs`.
- [x] 2.2 Add `FeedMode` and `AttachRequest.feed`, and `EndReason::Lost`, in `protocol.rs`. In `attach.rs`, build the `Feed` before the attach commit and handle `Lagged` as `Lost`.
- [x] 2.3 Make the client's `observe()` ask for `every` and `next()` report `Lost` as "fell behind the server while waiting on pane P". Add the `Lost` arm to the UI, handled as a dropped stream.
- [x] 2.4 Verify slice 2's checks: consecutive revisions on `every`, coalescing on the default feed, `lost` under a temporary `RING = 2` probe (then revert), and unchanged waits and UI.

Follow-up verification after slice 5 passed on macOS with a temporary `RING = 2` build. A paused HTTP reader received only consecutive revisions 4 through 9 after seed 3, then `ended` with `data.reason: lost`, after 80 large independent snapshots were published. It did not receive the retained tail revisions, and its attachment disappeared before the client closed. A temporary local proxy paused actual upstream reads for a real `ship pane wait --status exited` during another 80 publications, then forwarded the server's real events without synthesizing SSE data. The server emitted `lost`; the CLI exited 1 with empty stdout and a pane-specific `fell behind the server` error, not `pane closed`. Its attachment was removed. The owned server and proxy used ports 43336/43337 and isolated config/state/home; shutdown left no attachments.

`RING` was restored to 256 and verified identical to committed source. Debug/release builds, formatting, workspace/all-targets Clippy with warnings denied and strict OpenSpec validation passed. The normal feed probe passed again: 20 consecutive every-feed revisions, 51 commits coalescing into 7 latest-feed states, requested/plain waits, send --wait, real PTY UI reconnection and shutdown. Evidence: `target/remaining-k2oyr7kf/{raw-summary,raw-events,cli-summary,cli-events}.json` and `target/slice5-feeds-c78jciec/`; temporary probe: `target/remaining-probe.py`. No implementation changes remain. This completes the overflow check recorded as incomplete in the earlier slice notes. Linux remains unverified.

## 3. Close on exit

- [x] 3.1 Add `ExitBehavior::closes`, and in the `PaneEvent` handler commit the exit and then, when it closes, the close as `Remove<Pane>` does.
- [x] 3.2 Add `exit_behavior` to `SetPane` and `pane::Set`, applied in the `Set<Pane>` handler and passed through in `execute.rs`.
- [x] 3.3 Verify slice 3's checks: `exit` closes a split and a lone pane, `keep` stays and reads back, `wait --status exited` on a closing pane prints the status, a plain wait fails with the exit code, `keep_failed` for code 0, code 3 and a signal, and `set --exit-behavior` on running and exited panes from the CLI and a key binding.

Slice 3 verified on macOS with an owned temporary server on port 43332 and isolated config, state and home directories. The probe captured consecutive exit/close revisions, selection moving to the sibling and its frame expanding to 100×30, and a lone shell leaving an empty selected tab. Kept panes retained output and history; closing-pane waits returned or reported code 4; `keep_failed` closed code 0 and kept code 3 and SIGTERM. CLI edits and real PTY key presses changed running panes and only stored changes on exited panes. HTTP null/no-op edits kept the revision and combined edits made one revision. Formatting, workspace/all-targets Clippy with warnings denied, debug/release builds and strict OpenSpec validation passed. Evidence is in `target/slice3-4e7j7w_k/`; the temporary probe is `target/slice3-probe.py`, both ignored by Git. Linux is unverified. Slice 2 task 2.4 remains incomplete.

## 4. History size on a live pane

- [x] 4.1 Add `SessionCommand::ScrollbackBytes` and `SessionHandle::set_scrollback_bytes` to `vendor/ratatui-ghostty/src/session.rs`.
- [x] 4.2 Add `PaneCommand::Scrollback`, and `scrollback_bytes` to `SetPane` and `pane::Set`. Send the command from the `Set<Pane>` handler whenever the request supplies the value, independently of whether shared state changes (Cyan-approved refinement). Document raising and lowering in `ship pane set --help`.
- [x] 4.3 Verify slice 4's checks: lowering drops the oldest lines at once, raising keeps history and lets it grow, and the help text says both.

Slice 4 verified on macOS with an owned temporary server on port 43334 and isolated config, state and home directories. After `seq 1 700000`, lowering from 10,000,000 to 1,000,000 bytes without new output reduced retained numeric lines from 13,849 (starting at 686152) to 847 (starting at 699154). Raising to 50,000,000 kept all 847 lines, then retained them alongside 20,000 new lines. HTTP changes and real PTY key presses also pruned or preserved history as requested. Combined HTTP edits committed once; absent, null and unchanged values kept the revision, and invalid values were rejected. Kept exited panes accepted history changes, including zero clearing history while preserving visible output. `pane set --help` describes raising and lowering. Formatting, workspace/all-targets Clippy with warnings denied, debug/release builds and strict OpenSpec validation passed. After Cyan approved replacing field-by-field change detection with `Pane` equality and forwarding every supplied history value, both slice 3 and slice 4 probes passed again. Latest evidence is in `target/slice4-s8kbvhsw/` and `target/slice3-4_gdr2ka/`; the temporary slice 4 probe is `target/slice4-probe.py`, all ignored by Git. Port 43333 was occupied and left untouched. Linux is unverified. Slice 2 task 2.4 remains incomplete.

## 5. Docs and specs

- [x] 5.1 Update the guide in `docs/site/`, following `docs/agents/docs.md`. Each item goes on the page whose scope fits it:
  - `configuration.md`: rewrite "Server settings" around `[server.defaults.pane]`, covering the per-pane history size and what raising or lowering it with `ship pane set --scrollback-bytes` does.
  - `panes.md`: rewrite the intro's "its pane stays open" for exit behavior, with `close`, `keep` and `keep_failed`.
  - `scripting/reading-output.md`: P2's agent workflow, which creates a pane with `--exit-behavior keep`, reads it, then closes it.
  - `saved-tabs.md`: panes keep their exit behavior and history size across a restart.
- [x] 5.2 Remove the "Not built yet" line about closing panes on exit from `README.md` and from the hand-written landing page `website/src/landing/body.html`. Move the landing page's config example `shell` from `[server]` to `[server.defaults.pane]`. The README no longer has a config example.
- [x] 5.3 Check the delta specs under `specs/` against the real output from slices 1 to 4, and the guide's examples against captured output. `openspec validate pane-defaults --type change --strict` passes.

Slice 5 verified on macOS. The guide covers the defaults table and migration, exit behavior, history resizing, the keep/wait/read/close workflow, and restored settings. The README and landing page no longer list automatic closing as unbuilt. The overview and landing-page CLI demos now keep their short-lived panes and wait before reading; their displayed output and the landing copy-button payload match fresh captures. Configuration-page TOML blocks and the landing config pass `ship config check`; old `[server] shell` and `scrollback_bytes` diagnostics name line 2, column 1. Fresh captures confirm defaults apply only to new panes, create overrides win, restart preserves pane settings while starting fresh shells without old output, and old saved files use built-in settings rather than current config defaults. All seven delta specs were reviewed against implementation and runtime evidence; the stalled-reader scenario was corrected to match immediate `lost` on detected lag, not draining retained revisions. Overflow itself remains unverified in task 2.4.

Evidence: `target/slice5-rie7s4pj/` (guide/config/restore), `target/slice5-feeds-ufinc7r1/` (consecutive revisions, coalescing, waits, UI reconnection and shutdown), `target/slice3-upxnl52h/` and `target/slice4-ixmhfji_/` (rerun exit/history workflows). Temporary probes are `target/slice5-probe.py`, `target/slice5-feeds-probe.py` and `target/slice5-site-check.py`; `target/slice5-spec-review.md` records the spec comparison. Formatting, workspace/all-targets Clippy with warnings denied, debug/release builds and strict OpenSpec validation pass. Website formatting and Astro checks pass; static builds pass at the root and at `/ship/`, with new links and anchors verified in the generated HTML. The builds emit an empty `i18n` collection warning. All probe paths are ignored by Git. Servers used owned temporary state/config/home directories on ports 43332, 43334, 43335 and 43336; 43337 was an owned local UI proxy. Linux is unverified. Stopped before task 6.1.

## 6. Size

- [x] 6.1 Report the change's implementation Rust and test line counts separately.

Measured from planning commit `886140085559189565903c2e2f8f88d3b799aebf` to slice 5 commit `b76d16b960fe1144ad6e323b02ab3b132b6b97bb`. `cloc` 2.10 excludes comments and blank lines; Git physical-line additions/removals are separate.

| Rust category | Code before | Code after | Net code change | Physical lines added / removed |
| --- | ---: | ---: | ---: | ---: |
| Ship-owned implementation, `crates/*/src/**/*.rs` | 11,254 | 11,468 | +214 | +382 / -123 |
| Modified vendored implementation, `vendor/ratatui-ghostty/src/session.rs` | 1,105 | 1,120 | +15 | +19 / -0 |
| Authored test Rust for this change | 0 | 0 | 0 | +0 / -0 |

Total net implementation code added: 229 lines, including the vendored setter. Physical Rust diff: +401 / -123, net +278. Ship-owned implementation totals 13,604 physical lines including comments/blanks, versus 13,345 before, below the roughly 20,000-line ceiling by either measure. There are no inline tests in the counted implementation and no changes to Rust test files. Existing upstream vendored tests are unchanged and excluded, as are temporary Python probes, experiments, examples, docs, TOML and external/generated dependencies. Reproduce with `python3 target/remaining-size.py`; per-file counts, both cloc outputs and scope are in `target/remaining-size/`. Scripts and captures are ignored by Git. All 21 tasks are complete; archiving remains a separate action.
