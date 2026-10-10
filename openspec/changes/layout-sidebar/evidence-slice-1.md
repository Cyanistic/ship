# Slice 1 evidence

Slice 1 is complete through tasks 1.1–1.6; slice 2 has not started. The subsequent approved on-tab geometry correction and its full rerun are recorded in [evidence-slice-1-correction.md](evidence-slice-1-correction.md); that record supersedes the parallel replica-geometry representation in this historical evidence. The final verification section below records completion and Cyan's approved legacy-client exception. Earlier probe findings and the compatibility-blocker section are historical evidence, not the current policy or final acceptance result.

## Approved decision

Architecture A-4 and program P-1 record Cyan's approval ("yeah i think i agree with herdr too. let's do that.", followed by "YES PLEASE YOU ADD IT FIRST THEN DISPATCH THE WORKER."). Both the program PTY and Ghostty emulator dimensions are clamped independently to at least 1 column and 1 row on startup and resize. Published frame/content geometry remains actual squeeze-to-fit geometry, including zero dimensions. Cells and cursors are clipped to actual content; neither is drawn when either content dimension is zero. The 1x1 actual-content minimum for new splits/manual resizes is unchanged.

The earlier last-positive-emulator/zero-PTY proposal was not adopted. The worker must finish cursor-clipping proof and record the approved-floor startup/shrink/enlarge results before completing task 1.1, then implement and verify tasks 1.2–1.6. No new fallback is authorized.

## Context read

- Ran `openspec list --json`, `openspec status --change layout-sidebar --json` and `openspec instructions apply --change layout-sidebar --json` in `/Users/cyan/Documents/projects/ship`.
- Root: this repository. Schema: `spec-driven`. Apply: `ready`, 0/26 complete. Repo-local edit scope. Context requires vertical slices; operation guidance requires stopping after the current slice.
- Read every returned context file, including all eight change specs, proposal, design and tasks, completely.
- Read locked `design/product.md`, `design/architecture.md`, `design/program.md` under the change, all ten main specs, `AGENTS.md`, `docs/agents/code.md`, and global style/engineering-philosophy skills.
- Cyan's current instruction authorizes implementation despite historical planning-only prose. It does not authorize choosing a new zero-content fallback.

## Temporary probe artifacts

All probe code, manifests, binaries and detailed logs are outside the workspace:

`/tmp/ship-layout-slice1-3pFjpy/`

- `src/main.rs`, `Cargo.toml`, `Cargo.lock`: temporary probe source and resolved dependencies.
- `geometry.log`: initial `Constraint::Ratio` mapping and rendered buffer output.
- `geometry-fill.log`: proportional `Constraint::Fill` mapping, exhaustive small-area containment/overlap checks and rendered buffer output.
- `ghostty.log`: direct Ghostty construction/resize results.
- `pty-{0x0,0x1,1x0,1x1}-{resize,initial}.log`: real macOS PTYs and vendored wrapper, capture/cursor output, restored screens and child cleanup results.

Dependencies match the relevant repo versions: ratatui 0.30.2, ratatui-core 0.1.2, ratatui-widgets 0.3.2, portable-pty 0.9.0 and libghostty-rs revision `8953a740bc378cec3e07e1f6ca949f0595eab19b`. Other transitive dependencies were resolved independently for the temporary crate.

## Commands and observations

### Geometry

```sh
cargo run --manifest-path /tmp/ship-layout-slice1-3pFjpy/Cargo.toml
```

The final run exited 0. The probe exercised widths 0–101, heights 0–33, first-child shares 0.5 and 0.7, a horizontal split whose second child is split vertically, and shrink/enlarge without changing fractions. It checked frame/content containment, shared one-cell edges and complete area coverage. `Block::bordered().merge_borders(MergeStrategy::Exact)` rendered joined junctions into the actual ratatui buffer. At 81x25, the final frames were:

```text
A: x=0,  y=0,  width=41, height=25
B: x=40, y=0,  width=41, height=13
C: x=40, y=12, width=41, height=13
```

Important mapping finding: two `Constraint::Ratio` values summing to one with `Spacing::Overlap(1)` left an unused trailing row/column. Proportional `Fill` constraints covered the whole area, preserving shared edges. The temporary Fill mapping used weights summing to 32768; this proves the library mechanism at these samples, not a final production mapping for every f32 fraction or future cell-based resize. No geometry mapping was added to production.

The rendered buffer is probe evidence only. A real rendered Ship client was not exercised, and no slice 2 rendering claim is made.

### Ghostty's zero-dimension boundary

```sh
/tmp/ship-layout-slice1-3pFjpy/target/debug/ship-layout-slice1-probe ghostty
```

Exited 0:

| Size (cols x rows) | `Terminal::new` | `Terminal::resize` from 80x24 | Restore to 81x25 |
| --- | --- | --- | --- |
| 0x0 | `Err(InvalidValue)` | `Err(InvalidValue)` | `Ok(())` |
| 0x1 | `Err(InvalidValue)` | `Err(InvalidValue)` | `Ok(())` |
| 1x0 | `Err(InvalidValue)` | `Err(InvalidValue)` | `Ok(())` |
| 1x1 | `Ok(())` | `Ok(())` | `Ok(())` |

### Real PTY and wrapper

Ran the temporary binary for each table size in both `resize` and `initial` modes. Each final invocation was bounded by a Python `subprocess` harness with a five-second deadline and its own process group. All eight final probe processes exited 0; that exit denotes successful probe execution, not successful zero-size emulator construction.

For already-started 80x24 wrapper sessions:

- macOS PTY resize accepted 0x0, 0x1 and 1x0; `get_size` returned those exact values.
- The wrapper reported those dimensions and captured zero cells and no cursor, without an event error.
- The shell stayed alive. After restoration its retained screen contained `stty size` results `0 0`, `1 0` or `0 1`, respectively, followed by `alive`, and then `25 81` and `alive`.
- Capture's zero-cell iteration did not execute the client's modulo/division arithmetic. Cursor clipping did not produce a position for the empty screen.
- The positive 1x1 control also worked and restored to 81x25.
- Resized-session children exited under hangup and were reaped according to `try_wait`.

For initially zero-sized wrapper sessions:

- The PTY opened, but the wrapper emitted `Error(Terminal(InvalidValue))` and `Exited`.
- Enlargement did not recover the dead wrapper or resize the PTY.
- Initial-zero shell children did not exit under the probe's portable-pty kill during its short polling window; the harness terminated their owned process groups. This is not evidence for Ship's separate teardown contract.

A first scratch run waited indefinitely for one initial-zero child after kill and hit the tool's 120-second deadline. The final harness used bounded polling and owned-process-group cleanup. A final `ps -axo pid,ppid,command` search found no probe binary or probe shell remaining. No existing Ship server or user programs were touched.

## Root cause and ownership trace

1. `crates/ship-server/src/pane.rs:389–391` records requested dimensions and forwards them to `SessionHandle::send_resize`.
2. `vendor/ratatui-ghostty/src/session.rs:556–564` discards both emulator and PTY resize results, then updates the shared requested dimensions.
3. The underlying Ghostty terminal rejects zero dimensions, proven directly above. The wrapper's empty render and recovered output therefore do not prove the emulator accepted a zero-sized grid; it retained its earlier positive grid.

The new layout will legitimately produce zero-content panes during terminal shrink. Choosing what the emulator does at that boundary is the explicit architecture R-2/task 1.1 approval gate. Merely observing that today's wrapper hides the error is not approval of that policy.

## Unverified and remaining

- No rebuilt temporary Ship server, route/OpenAPI checks, attachment geometry comparisons, real CLI split/close workflows, protocol 8 checks or implementation fmt/build/Clippy runs were performed. They remain tasks 1.2–1.6 after approval.
- No real rendered-terminal check was performed.
- macOS probe results only. Linux and Windows are unverified.
- No permanent authored tests, generated-file manual edits, commits, pushes or releases.

## Reuse, new code and proof

- reused: ratatui layout/border/render primitives, portable-pty and the existing vendored Ghostty wrapper.
- new code: outside-workspace disposable probes only; this evidence file is the sole repo change.
- proof: actual Ghostty error returns, real PTY dimensions and restored `stty size` output, exhaustive bounded-area ratatui assertions and rendered buffer logs. These establish the gate, not slice completion.

## Task 1.1 approved-floor verification

Re-read current apply context and locked product/architecture/program papers. Apply is ready; scope is slice 1 only. No new policy choice is needed.

Rebuilt `/tmp/ship-layout-slice1-3pFjpy/` with `cargo build --manifest-path .../Cargo.toml` and ran its real PTY/wrapper binary through a five-second `subprocess.run` deadline for 0x0, 0x1, 1x0 and 1x1, in both initial and resize modes. Both PTY and emulator receive independently floored dimensions. All eight runs exited 0, recovered `25 81` and `alive`, and passed explicit cursor clipping assertions against actual content. Raw wrapper cursors can exist at zero content; the filtered position cannot. Zero-content buffers are empty. Logs: `cursor-build.log` and `cursor-floor-{cols}x{rows}-{initial,resize}.log` in that temporary directory. Earlier geometry-fill probe evidence remains applicable. Task 1.1 is accepted for macOS; Linux/Windows remain unverified.

## Tasks 1.2–1.5 implementation verification

`cargo check --workspace`, `cargo check -p ship-core --no-default-features`, `cargo check -p ship-core --features clap`, `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` passed on the macOS host. The temporary contract crate at `/tmp/ship-layout-slice1-3pFjpy/contract/` constructs and serializes `ship_server::openapi()` without starting a listener. Recursive `Layout`/`Split` references terminate using utoipa's `no_recursion` field annotation, not a second hand-written layout schema. Its scalar ratio round-trip and invalid/nonfinite ratio assertions passed. `openapi.json` is the generated inspection artifact, outside the repo.

`python3 /tmp/ship-layout-slice1-3pFjpy/workflow.py` passed against an owned rebuilt debug server on an ephemeral loopback port, ending with `WORKFLOW PASS` and `OWNED SERVER/PROGRAM CLEANUP PASS`. `workflow.log`, `server-workflow.log`, `tab-get-before.json`, `workflow-last-replica.json` and `workflow-events.json` hold commands, actual responses, geometry and screens. The temporary Rust contract binary exercised ordinary shared-DTO client methods and key-scope `execute` (down split anchored to the selected pane); it printed `ORDINARY CLIENT + KEY ANCHOR/DIRECTION PASS`.

Verified real CLI cases: explicit down/right anchors, tab-only largest and layout-order ties, empty tabs, `SHIP_PANE_ID` anchor and explicit `--tab` precedence. At 81x25, `[A | [B / C]]` publishes content sizes A=39x23, B=C=39x11, and `stty size` reports `23 39`, `11 39`, `11 39`. Two viewers reduced the tab to 61x19, with A=29x17 and B=C=29x8. Moving the smaller viewer to another tab restored the first to 81x25 while the other independently used 31x11. Sizes matched every positive content rectangle; zero dimensions were floored independently to one.

Shrink/enlarge exercised 2x2, 0x0, 0x1, 1x0, 1x1, 9x7 and 81x25. Every frame/content rectangle stayed contained, every shell stayed alive, pane IDs/tree/ratios were byte-equivalent on tab inspection, screen grids and `stty size` matched the approved internal floor, and the other tab was unaffected. An empty tab viewed at 0x0 started its first real program at 1x1 and recovered to 81x25. An unviewed tab kept its last program size.

Refusing an undersized split left structure and revision unchanged. Its command would create a marker and sleep; the marker did not appear, so no program started. Failed directory/program creation and missing/invalid anchors likewise preserved structure. Closing A collapsed `[A | [B / C]]` to `[B / C]`, selected B, ended only A's tracked PID, and preserved B/C. Closing C chose B; closing the last B selected the tab. Closing a right leaf next to a same-axis subtree selected that subtree's rightmost leaf. Program cleanup was checked by tracked PIDs and server teardown logs. Moved surviving selection and ancestor fallback remained intact.

Wire validation used Python's `Draft202012Validator` against the actual generated OpenAPI document for each successful API response, every streamed event/replica, and documented structured errors. This exposed a create-body rejection/schema mismatch; create now uses the existing `move_tab` decode-after-JSON pattern for a consistent 422 `AppError` on malformed targeting. Existing program/directory `Validation` failures remain HTTP 400 and are documented as such. No new fallback or scope expansion was needed.

## Historical task 1.6 compatibility blocker: stopped for a decision

The slice is **not complete**. Tasks 1.1–1.4 are checked; 1.5 and 1.6 remain unchecked. Close implementation and the recorded close workflow passed, but the remaining multiple-affected-viewer and actual ancestor-removal variants were not completed before the blocker. Slice 2 was not started. No README update, documented-example run, release build or rendered Ship-client exercise is claimed.

To verify mixed versions with an actual old binary, copied the checked-in HEAD sources with `git archive HEAD Cargo.toml Cargo.lock crates vendor` into the disposable `/tmp/ship-layout-slice1-3pFjpy/baseline7/` directory (not a Git worktree), built `-p ship`, and retained `/tmp/ship-layout-slice1-3pFjpy/ship7`. The current rebuilt binary was retained as `ship8`. Rebuilt the repository debug binary afterward. All builds used only this macOS host; no other platform or live user server was used.

`python3 /tmp/ship-layout-slice1-3pFjpy/protocol.py` proves the blocker in **both** directions against owned ephemeral servers:

| Server/client | Command | Result |
| --- | --- | --- |
| 8/7 | `server status` | Exit 1, empty stdout, incompatible protocol 7 diagnostic |
| 8/7 | bare client | Exit 1 before attach, incompatible protocol 7 diagnostic |
| 8/7 | `--server-url URL tab list` | Exit 0, `{}`; sends GET `/api/v0/tabs` without health |
| 7/8 | `server status` | Exit 1, empty stdout, incompatible protocol 8 diagnostic |
| 7/8 | bare client | Exit 1 before attach, incompatible protocol 8 diagnostic |
| 7/8 | `--server-url URL tab list` | Exit 0, `{}`; sends GET `/api/v0/tabs` without health |

Artifacts: `build-baseline7.log`, `protocol.py`, `protocol.log`, `mixed-server8-client7.log`, `mixed-server7-client8.log`. Both owned servers remained alive during incompatibility checks and were then stopped normally with their matching binaries. No existing server was restarted.

Root cause, two layers up: shared endpoint methods intentionally leave identity/startup policy to the application; `crates/ship/src/main.rs::dispatch` routes ordinary actions through `connect`, and `connect` validates health only for non-explicit targets. An explicit URL skips health entirely. The old protocol-7 build has the same omission. `server stop` also bypasses the application's `health` wrapper. Fixing current protocol-8 preflight can follow the already-approved health contract, but cannot retroactively change an existing protocol-7 binary. The server has no request-level protocol marker that would distinguish that old request before layout access.

A new required request protocol marker/server guard is a public-interface decision, not an authorized mechanical consequence of replacing the layout. Accepting an already-built protocol-7 explicit command as an exception would narrow task 1.6's acceptance. Neither was silently adopted. Parent/Cyan must settle which claim to require before continuing.

Final pre-pause checks: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `git diff --check` passed. Debug workspace build and core checks with/without clap passed as recorded above. Release, real rendered Ship UI, Linux and Windows are unverified. No permanent authored tests, manually edited generated files, commits, pushes or releases were added.

reused: ratatui Fill/Overlap and Block geometry, utoipa recursive schema annotations, existing commit/runtime cleanup/ancestor repair, shared client DTOs and pane wrapper.
new code: pane-bearing binary layout and fraction, wire geometry, server geometry/split/close/successor edits, target preservation, internal terminal floor and selected-screen clipping. These replace the missing domain behavior; they do not create a second layout owner.
proof: real CLI/API/SSE/stty/PID workflow and JSON Schema validation, temporary ratio/OpenAPI/floor probes, actual mixed-version binaries and request logs, plus host debug/core/fmt/Clippy checks. The mixed-version evidence is a failed acceptance, not a pass.

## Final tasks 1.5–1.6 verification and approved acceptance adjustment

Cyan explicitly approved reusing the application health helper for current explicit ordinary commands and stop, with no autostart, and documenting the already-built protocol-7 exception instead of introducing a request marker/server guard. `design.md`, `tasks.md` and `specs/health-exchange/spec.md` now express that acceptance precisely. No new product or wire decision was made in this continuation.

### Changes in this continuation

- `crates/ship/src/main.rs`: `connect` calls the existing `health` helper for explicit flag/environment targets. `dispatch` validates through the same helper before `commands::stop_server`, without calling local startup. Ordinary endpoint methods remain policy-free. The two-layer cause is the dispatch/connect policy omission above those methods, not a server route or DTO problem.
- `crates/ship-core/src/command.rs`: pane-create help explains the split anchor and tab-only largest/empty behavior, overriding the shared `TabTarget` help only for creation. No command or argument shape changed here.
- `README.md`: current slice status, right/down create and close examples, layout inspection, selection/program lifetime, minimum/floor handling and protocol-8 paired upgrades. It explicitly does not claim simultaneous rendering/focus/sidebar.
- Change contract and this evidence: approved legacy exception and final close/compatibility/build results. Existing slice-1 close/commit/successor/runtime cleanup implementation required no further change.

All detailed artifacts remain in `/tmp/ship-layout-slice1-3pFjpy/`.

### Close behavior and actual ancestor removal

`python3 /tmp/ship-layout-slice1-3pFjpy/workflow.py > .../workflow-final.log 2>&1` exited 0 after adding the missing close variants to the disposable workflow:

- Two viewers selected on A in `[A | [B / C]]` both repaired to B. A third viewer selected on C remained on C. Only A's tracked PID ended; B/C remained alive and their expanded content matched `stty size`.
- Removing C repaired affected viewers to B. Removing the last B repaired all three to the owning tab, which had no layout/panes.
- Closing a right leaf beside a same-axis sibling subtree selected its right edge; closing a left leaf beside a same-axis subtree selected its left edge. Both surviving programs kept running. The mixed-axis case above selected the first child.
- Actually deleting an intermediate ancestor removed its nested selected pane/tab. Pane-selected and tab-selected viewers repaired to the surviving root; a viewer of the root's own pane and that program were unaffected. Deleting a top-level ancestor with nested pane/tab selections repaired all affected selections to nothing and ended its programs. This is ancestor deletion, not just deletion of the selected child. The earlier moved-surviving-selection case still passed.
- Final output: `MULTIPLE AFFECTED VIEWERS + UNAFFECTED SELECTION + LAST-PANE TAB FALLBACK PASS`, `SAME-AXIS SUCCESSOR BOTH CLOSED SIDES PASS`, `ACTUAL ANCESTOR REMOVAL: NEAREST SURVIVOR + TOP-LEVEL NONE + UNRELATED PROGRAM PRESERVED PASS`, `WORKFLOW PASS` and `OWNED SERVER/PROGRAM CLEANUP PASS`.

The final workflow also reran the existing successful schema/ordinary API/key-targeting, largest/tie/empty, no-space/no-start, shrink/floor/recovery and viewer-isolation checks. It validated actual successes, documented structured errors and every SSE event/replica against `openapi.json`. `tab-get-before.json`, `workflow-last-replica.json`, `workflow-events.json` and `server-workflow.log` were refreshed. At 81x25 before closing A, `stty size` again returned A=`23 39`, B/C=`11 39`; after closing A, B/C returned `11 79`. Positive terminal sizes equaled published content; each zero dimension used the approved independent floor.

### Request-log compatibility proof

`python3 /tmp/ship-layout-slice1-3pFjpy/protocol.py > .../protocol-final.log 2>&1` exited 0. It now asserts the approved acceptance, rather than expecting both historical bypasses:

- Actual protocol-8 client against the owned protocol-7 server, via both `--server-url` and `SHIP_SERVER_URL`: status, bare attach, tab list/create, pane create/close and stop each exited 1 with empty stdout and incompatibility diagnostics. Each actual server request-log delta contained exactly `GET /health`; no layout, attach or shutdown request occurred, and the server remained alive until its matching client's cleanup.
- Actual legacy protocol-7 client against the owned protocol-8 server: status and bare attach rejected on health. Explicit ordinary requests did bypass health, as documented. In particular legacy tab creation reached the server and mutated it even though the old client then failed decoding the new response (`missing field panes`). This is an observed limitation, not a compatibility pass. Both URL sources were exercised.
- Six temporary health responses (wrong service, protocol 7, invalid JSON, missing field, wrong field type and HTTP 503) were exercised with both URL sources across tab list/create, pane create/close, bare attach, stop and status: 84 failures with empty stdout. Each captured request delta was exactly `GET /health`, never layout/attach/shutdown. `health-rejection-requests.json` holds every command, request delta, exit and diagnostic.
- A refused explicit loopback target stayed unbound after ordinary, attach, stop and status attempts from both sources. All failed; no fallback/startup was used. The code's explicit and stop branches never enter `local::default_health`.
- Against a compatible owned protocol-8 server, flag/environment status returned one JSON line with `protocolVersion: 8`. Actual request logs showed health before tab create/get/close. A real PTY client attached after health (`GET /health`, `POST /api/v0/attach`) and detached with Alt-q, exit 0. Compatible stop showed `GET /health` before `POST /api/v0/server/stop`, then normal shutdown. This proves attach behavior, not slice-2 rendered appearance.

Artifacts: `protocol-final.log`, `protocol.py`, `mixed-final-server8-client7.log`, `mixed-final-server7-client8.log`, `health-rejection-requests.json`, `compatible-final-server8.log` and `compatible-attach-pty.bin`. All owned processes exited and were reaped.

The first revised protocol-probe run mistakenly expected legacy creation to decode successfully. It failed on `missing field panes` after reaching the new server, then cleaned up normally. The assertion was corrected to match the approved no-guarantee legacy acceptance; final proof is the subsequent passing run. A scratch clap help override also used the long flag name instead of its internal argument ID; the real `--help` run exposed that immediately. The override now uses `id`, and final help/workflow runs passed. Neither failed scratch attempt is counted as acceptance.

### Documentation, help and final host checks

- `python3 /tmp/ship-layout-slice1-3pFjpy/examples.py > .../readme-examples-release.log 2>&1` extracted the README's shell block and ran it verbatim with Bash/jq and the rebuilt release binary against an owned ephemeral release server. Both inspections showed the expected three-leaf then vertical two-leaf layouts; final close left `{}`. Release status returned protocol 8. Output: `VERBATIM README SHELL EXAMPLES + RELEASE HEALTH8 + EMPTY POST-CLEANUP TREE PASS` and `OWNED RELEASE SERVER CLEANUP PASS`. Artifacts: `readme-examples.sh`, `readme-examples-release.log`, `release-examples-server.log`; debug workflow inspections also remain in `readme-tab-before.json` and `readme-tab-after.json`.
- Actually ran `ship pane create --help`, `ship pane close --help`, `ship tab get --help` and `ship server stop --help`; outputs are `help-*.txt`. Create help now reports anchor/default, largest/empty/tab precedence and right/down options. Help generation is an execution check, not inferred from compilation.
- `cargo build --workspace`: passed (`final-debug-build.log`).
- `cargo build --workspace --release`: passed (`final-release-build.log`).
- `cargo check --workspace`, `cargo check -p ship-core --no-default-features`, `cargo check -p ship-core --features clap`: passed (`final-check.log`).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed (`final-clippy.log`).
- `cargo fmt --all -- --check` and `git diff --check`: passed (`final-fmt.log`).
- `CARGO_TARGET_DIR=/Users/cyan/Documents/projects/ship/target cargo run --manifest-path /tmp/ship-layout-slice1-3pFjpy/contract/Cargo.toml`: passed, `OPENAPI + RATIO PASS` (`final-contract.log`). Recursive OpenAPI serialization terminated and scalar ratio assertions passed; the document was generated, not manually authored or edited.

All final Rust changes were included in the last debug/release/check/Clippy/fmt and runtime runs. Owned implementation Rust under `crates/`: 6,966 lines in 39 files; authored Rust tests under `crates/`: 0 lines. Vendored Rust is reported separately: 3,139 lines. Temporary probes are not permanent authored tests.

Scope and residual risk: macOS host only; Linux/Windows remain unverified. No slice-2 simultaneous rendering, directional focus or sidebar work/check is claimed. Legacy protocol-7 explicit ordinary commands retain the approved limitation. No generated-file manual edits, permanent authored tests, commits, pushes or releases. No existing user server was contacted, stopped or restarted. A final read-only process check saw the pre-existing default-port server PID 86531 (started October 9), which was left untouched; all workflow servers used ephemeral ports and recorded/reaped their own PIDs.

reused: existing application `health`, local startup distinction, ordinary shared-DTO endpoint methods, close/successor/commit/runtime teardown and ancestor fallback, plus temporary workflow/OpenAPI probes.
new code: current explicit-command and stop preflight in application dispatch; command-specific help overrides. The missing policy was above endpoint methods; no endpoint guard, new protocol marker or backwards translation was introduced.
proof: real mixed-version binaries and exact request-log deltas, 84 bounded health rejection cases, compatible real PTY attach and shutdown ordering, multiple-viewer/ancestor close and PID/stty/schema workflow, verbatim release README examples and final host structural checks. Tasks 1.5 and 1.6 meet the approved acceptance; stop before slice 2.
