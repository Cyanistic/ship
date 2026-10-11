# Slice 1 correction: geometry on each tab

The approved correction is implemented and verified. Each recursive published tab carries optional geometry, present exactly when viewed. `Replica.geometry` is removed. The server keeps geometry-free source layouts and derives geometry at publication; clients read it rather than calculate layout. Tasks 1.1–1.6 retain their completion after the reruns below. Progress remains 6/26. Slice 2 has not started.

## Approval and scope

Cyan reviewed the existing `Replica { tabs, geometry map }` against recursive tabs with layout and optional geometry, computed from layouts/viewers when published. Cyan accepted fixing before commit ("perfect. can we fix that please or should we commit what we have here first?"), then authorized it ("sounds right. let's go for it!"). Architecture A-5 and program P-2 record the narrow amendment and reapproval. Original locked text and status remain preserved, with superseding amendment pointers.

This is a representation correction within the uncommitted slice. Protocol stays 8. The approved protocol-7 explicit-command limitation, actual zero-content geometry, independent internal PTY/emulator max(1) floor, and last-size behavior remain unchanged. No topology synchronization, new route, permanent tests, generated-file manual edits, commits or live-server restarts were added. The separate `Json<CreatePane>` review finding was not changed. Rendering/focus/sidebar work remains slice 2 or later.

Read `AGENTS.md`, `docs/agents/code.md`, style and engineering-philosophy skills, all OpenSpec apply context files, all three locked papers and the existing slice evidence. Ran list/status/apply; the repository root was selected, schema `spec-driven`, state `ready`, 6/26 complete. The explicit correction instruction controls this run rather than starting pending slice-2 tasks.

## Root cause and implementation

The ownership trace is two layers above the consumers:

1. Architecture decision 1 and the program skeleton put viewer-dependent geometry beside tabs in the replica, despite the server already deriving it from layout/viewers. That shape made `Tab` inspection omit geometry and required parallel lookup by tab ID.
2. `ServerState::replica` constructed the separate map; `commit` sized from that map. `attach::Progress::view` and the selected-screen renderer therefore resolved a tab and then independently indexed replica geometry. No client-side layout calculation was needed, and no mutable server geometry cache existed to preserve.

Correction files:

- `crates/ship-core/src/model.rs`: `Tab.geometry: Option<Box<TabGeometry>>`, defaulting to absent and omitted on serialization when unviewed. Boxed only to keep `Tab` compact; the JSON shape is ordinary optional geometry.
- `crates/ship-core/src/protocol.rs`: remove `Replica.geometry` and its schema hook; keep protocol 8.
- `crates/ship-core/src/geometry.rs`: remove the tab-keyed `geometry_schema`. Keep `TabGeometry`, pane geometry, largest-pane choice and rectangle schema.
- `crates/ship-server/src/state/geometry.rs`: `publish` walks recursive tabs once. It derives geometry for sizes present in the viewer-derived size map, copies viewed tabs and ancestors, and shares other `Arc<Tab>` branches. It returns a publication value, never mutating source tabs or retaining a cache.
- `crates/ship-server/src/state.rs`: `published_tabs` composes the existing `tab_sizes` with that projection. Replica/attach seed and tab list/get/rename/move responses use it. A newly created tab has no viewers or children, so its existing create response with absent geometry already has the same semantics. Commit resizes recursively from `replica.tabs`, the exact projection submitted for that commit, without recalculating geometry for PTYs.
- `crates/ship-server/src/attach.rs`: resolve the viewed tab and read that tab's optional geometry for watched pane IDs.
- `crates/ship-client/src/ui/draw.rs`: use `Selected.tab.geometry` for the existing selected-screen content clipping. No simultaneous drawing or focus change.
- `crates/ship-client/src/api.rs`: document recursive published geometry on tab-list results; existing shared-DTO methods are reused unchanged.
- `README.md`: explain optional per-tab geometry, nested viewed children and inspection omission when unviewed.
- Change planning: `design.md`, `design/architecture.md`, `design/program.md`, `proposal.md`, `specs/session-observation/spec.md`, `tasks.md` and the prior evidence pointer. Original paper skeletons stay as superseded history.

The first strict Clippy run identified `large_enum_variant` in the existing `Outcome`: inline optional geometry enlarged `Tab` to at least 360 bytes. Boxing just the optional geometry resolved it without changing the `Outcome` public enum or JSON. The failed diagnostic is retained as `correction/clippy-first.log`; all final checks passed.

## Artifact paths

Temporary code and raw evidence remain outside the repository:

`/tmp/ship-layout-slice1-3pFjpy/`

New correction logs are under `correction/`. The existing `workflow.py`, `protocol.py`, `examples.py` and contract crate were reused. Disposable geometry consumers were updated for nested tabs. `correction/workflow-before.py` and `correction/before.patch` preserve the prior workflow and pre-run tracked diff; `correction/state-before.rs` and `geometry-before.rs` preserve the relevant old implementation. Existing generated `openapi.json`, workflow replica/events/tab inspections, request-log artifacts and stty/PID outputs were refreshed by reruns, not manually edited.

## Performance inspection

Before adopting publication copies, ran an outside-repo release microbenchmark at `contract/src/bin/publication_perf.rs`. It imports the actual server geometry/tree modules and constructs three-level branches, including layouts on unviewed ancestors. The old-map analogue clones tab handles and computes geometry by keyed lookup. A first proposed keyed-mutation COW projection took about 42.8 ms with 1,000 viewed branches, so it was not adopted. The implemented projection instead makes a single recursive walk.

Final benchmark command:

```sh
CARGO_TARGET_DIR=/Users/cyan/Documents/projects/ship/target \
  cargo run --release \
  --manifest-path /tmp/ship-layout-slice1-3pFjpy/contract/Cargo.toml \
  --bin publication_perf
```

`correction/perf-before.log` records the rejected keyed-mutation candidate. `correction/perf-after.log` records the implemented projection:

| Three-level branches | Viewed grandchildren | Panes per tab | Iterations | Old-map analogue | Actual recursive projection |
| --- | --- | --- | --- | --- | --- |
| 12 (36 total tabs) | 2 | 3 | 5,000 | 0.657 µs | 2.993 µs |
| 1,000 (3,000 total tabs) | 10 | 8 | 300 | 12.834 µs | 71.088 µs |
| 1,000 (3,000 total tabs) | 1,000 | 8 | 10 | 5,021.646 µs | 3,507.317 µs |

The probe also asserts source geometry remains absent, geometry is present exactly for viewed IDs, unmodified branches retain pointer identity, and viewed tabs/ancestors do not share their source `Arc`. This measures projection/allocation/geometry only, excluding viewer-size derivation, serialization, HTTP/SSE, scheduling and process resize. It is not an end-to-end responsiveness claim. The disposable crate resolves transitive dependencies independently; relevant geometry versions remain ratatui 0.30.2/ratatui-core 0.1.2. No performance framework or cache was added to production.

## Real CLI/API/SSE and program-size rerun

```sh
CARGO_TARGET_DIR=/Users/cyan/Documents/projects/ship/target \
  cargo run --manifest-path /tmp/ship-layout-slice1-3pFjpy/contract/Cargo.toml \
  --bin ship-slice1-contract
python3 /tmp/ship-layout-slice1-3pFjpy/workflow.py
```

Both exited 0. Logs: `correction/contract.log`, `correction/workflow.log`, `server-workflow.log`; generated schemas: `openapi.json`. The workflow launches its own rebuilt debug server on an ephemeral loopback port and cleans it up.

Every streamed Attached/State replica is checked against generated JSON Schema and a recursive semantic assertion: no top-level geometry, geometry exactly on viewed tabs, smallest area in each dimension derived from the same replica's viewer records, visible pane IDs in layout order, and contained frame/content rectangles. This checks consistency within each publication rather than combining fields from separate commits. The schema contains optional `Tab.geometry` and no `Replica.geometry`; recursive serialization terminates.

The reused real workflows passed again: explicit right/down anchors, key-scope anchor/direction through the ordinary Rust client, tab-only largest/tie/empty, `SHIP_PANE_ID` and explicit-tab precedence, undersized split refusal before start with unchanged source/revision and no marker program, failed creation cleanup, viewer isolation, close collapse, multiple affected/unaffected selections, same-axis successor in both directions, last-pane fallback, moved selection, actual ancestor removal and owned-PID teardown.

Sizes reran through 2x2, 0x0, 0x1, 1x0, 1x1, 9x7 and 81x25, with unchanged pane IDs/programs/layout fractions. Inspections compare source fields excluding only newly viewer-dependent geometry. At 81x25, `[A | [B / C]]` reports A content 39x23, B/C 39x11; actual `stty size` gives `23 39`, `11 39`, `11 39`. Two viewers constrain it to 61x19. Moving the smaller viewer away restores it to 81x25 and independently sizes another tab to 31x11. Zero dimensions floor independently at the terminal boundaries, while published rectangles remain actual zero dimensions. Empty-tab startup at zero and enlargement passed.

Added disposable nested checks passed:

- Three-level root/parent/child: a viewer on a child pane gives the child geometry at 73x21 while root and parent omit geometry.
- A quiet attach streams both child panes without another output change, and no unrelated screens.
- Root/child get, list, rename and parent move responses include the same child geometry as the committed observation. Moving the parent out and back preserves the child selection/geometry.
- Viewing an empty parent produces its own 19x9 geometry with no panes, without constraining the child. Moving that viewer to nothing removes parent geometry.
- Actual last-child-viewer socket detach removes child geometry from subsequent SSE publications and inspection. The child programs retain their last terminal size: the second vertically split pane still reports `9 71`.
- The temporary Rust contract uses the real `Client::attach` SSE decoder and shared DTOs for nested Attached/State, get/list/rename/move and view-to-none. It prints `CURRENT RUST CLIENT NESTED ATTACH/STATE/GET/LIST/RENAME/MOVE OPTIONAL GEOMETRY PASS`.

`correction/nested-after-detach.json` records the omission after detach. Overall outputs include `THREE-LEVEL ... PASS`, `WORKFLOW PASS` and `OWNED SERVER/PROGRAM CLEANUP PASS`.

## Current interactive-client consumer

```sh
python3 /tmp/ship-layout-slice1-3pFjpy/correction/ui-client.py
```

Exited 0 against another owned ephemeral debug server. The real current client ran in an 81x26 PTY; an external view update selected a grandchild pane. Its emitted terminal bytes contained `NESTED_ON_TAB`, confirming the selected-screen consumer reaches nested on-tab geometry. For an actual 0x0 reported area, emitted cursor control hid the cursor and inspected geometry remained zero. Restoration to 81x25 plus fresh program output produced `NESTED_RECOVERED`. Alt-q exited 0; the owned server/client were cleaned up.

Artifacts: `correction/ui-client.log`, `ui-server.log`, `ui-positive.bin`, `ui-zero.bin`, `ui-recovered.bin`. This is actual interactive-client execution and emitted terminal-output proof, not a screenshot review or a claim about future simultaneous rendering/sidebar appearance.

## Compatibility and documentation reruns

```sh
cp target/debug/ship /tmp/ship-layout-slice1-3pFjpy/ship8
python3 /tmp/ship-layout-slice1-3pFjpy/protocol.py
python3 /tmp/ship-layout-slice1-3pFjpy/examples.py
```

All exited 0. `correction/protocol.log` records actual protocol-7/8 binary pairs, flag/environment preflight, 84 malformed/wrong-service/wrong-protocol/status failures with only health requests, missing-target no-autostart, compatible ordinary/PTY attach/shutdown ordering and normal owned-process cleanup. Current protocol-8 clients reject before layout/attach/shutdown; legacy 7 still bypasses health on explicit ordinary commands with no compatibility guarantee. No exception was narrowed or removed.

`correction/examples.log` records the README shell block extracted and executed verbatim with the rebuilt release binary on an owned ephemeral release server: three-leaf create, close collapse, empty final tree and protocol-8 status. Actual help commands for pane create/close, tab get and server stop also passed; outputs are `correction/help-*.txt`.

## Final structural checks

All passed on this macOS host:

- `cargo build --workspace` (`correction/build-debug.log`).
- `cargo build --workspace --release` (`correction/build-release.log`). This is a local build, not a release or deployment.
- `cargo check --workspace`, `cargo check -p ship-core --no-default-features`, `cargo check -p ship-core --features clap` (`correction/check.log`).
- `cargo clippy --workspace --all-targets -- -D warnings`, plus ship-core with no default features and with clap (`correction/clippy.log`).
- `cargo fmt --all -- --check`, `git diff --check` (`correction/fmt.log`).
- `openspec validate layout-sidebar --strict --json` (`correction/openspec-validation.json`): one change passed, zero issues.
- Read-only search over implementation and generated schema: no replica geometry property, keyed replica lookup or `geometry_schema`; `Tab.geometry` is the only tab geometry field.

Reran the existing geometry probe and all eight real PTY/floor/cursor cases with bounded subprocess deadlines. Logs: `correction/geometry.log`, `floor-{0x0,0x1,1x0,1x1}-{initial,resize}.log`. They passed containment/overlap, unchanged proportions, startup/shrink/restoration and approved cursor clipping. Those probe mechanisms were unchanged by the representation correction.

## Uncertain decisions and residual risk

No additional product, interface, data-model or security decision was required. Boxing optional geometry is a Rust storage detail, not a JSON/protocol change. Deferrable appearance choices and all later-slice behavior remain untouched.

macOS only. Linux and Windows are unverified. No screenshot-based appearance review or slice-2 simultaneous rendering/focus/sidebar verification is claimed. Performance numbers are a bounded projection microbenchmark, not a latency guarantee. Legacy protocol-7 explicit ordinary commands retain the approved compatibility limitation. Source `Tab` and published `Tab` reuse one named type; server construction and projection keep source geometry absent, and the temporary probe proves that for the exercised trees. No independently mutable geometry state or persistent projection was introduced.

All workflow servers used ephemeral ports. Existing user servers were neither contacted nor restarted. Temporary servers/clients and tracked programs exited and were reaped. A final read-only process scan saw the existing default-port server PID 65222; it was left untouched. Owned implementation Rust under `crates/` is 6,985 lines in 39 files, versus the prior slice record's 6,966: net +19 lines for this correction. Permanent authored Rust tests remain 0 lines. Parent review of the actual diff remains the completion handoff; no commit was made.

reused: existing recursive `Tab`/`Tabs`/`Arc` types, layout-to-geometry calculation, viewer-size derivation, commit/revision ordering, PTY resize commands, attach watches, ordinary shared-DTO client methods, selected-screen clipping and outside-repo probes.
new code: optional on-tab geometry and a narrow recursive publication projection, plus recursive sizing and coherent tab responses/consumers. The previous representation required a parallel lookup; this correction removes it without a new framework.
proof: host builds/core variants/fmt/Clippy/strict OpenSpec, actual nested CLI/API/SSE/schema/stty/close/detach workflow, current Rust and PTY client consumers, actual mixed-version preflight/legacy exception, verbatim release examples and measured projection costs.
