# First terminal slice

Build one real interactive terminal across Ship's separate client/server processes as a later terminal checkpoint. The initial CLI/server/client scaffolding and health exchange are implemented; the terminal behavior in this plan is not. The broader [product brief](../../product-brief.md) remains revisable.

The current [session-structure design](session-structure.md) records recursive tabs, typed IDs and actors. It proposes a metadata roundtrip before PTYs; that next scope still needs approval. The [transport handoff](../research/terminal-transport-handoff.md) supersedes the historical patch direction for both structural and terminal state. The [Wayfinder map](../../.scratch/first-terminal-slice/map.md) is a historical decision trail, not the current implementation contract.

## Later terminal checkpoint

Start a local server and attach a local UI using the same `ship` executable in different modes. Interact with a real shell; smoke-test Pi and Neovim; resize; close the terminal; disconnect and reconnect the UI without interrupting server-owned work; shut down the server deliberately. Exact CLI spelling is still open.

Evidence should include keyboard input, alternate-screen entry/exit, correct terminal resizing, coherent screen updates, connection recovery, and restoration of the outer terminal on client exit. Validate Linux and macOS deliberately; record unavailable platform checks rather than claiming parity. macOS dependency tests, upstream example use and scratch screen/patch checks passed; the actual Ship client/server checkpoint has not been implemented or verified.

No splits, tabs, saved restart restoration, agent awareness, notification/audio implementation, remote attachment, or automation command catalog is required for this checkpoint. A server survives client disconnection, not its own process or machine restart.

Configuration and plugin systems are also deferred: use fixed defaults and only minimal operational/exit bindings. No config file, reload machinery, plugin runtime, manifests or registry. Local presentation actions stay in the client; shared session/process actions execute on the server. This does not settle where future configurable bindings are stored/interpreted. See [ownership precedents](../research/config-plugin-ownership.md).

## Code organization and development rhythm

```text
crates/
├── ship/           # binary, CLI parsing, startup and composition
├── ship-client/    # UI, local view state, input forwarding and state application
├── ship-server/    # processes, PTYs, terminal state and authoritative session behavior
└── ship-core/      # shared types, traits, utilities and helper logic
```

The app calls client/server entry points; both depend on core, not each other's internals. Helpers used by only one crate stay there until actually shared.

Use fine-grained coherent commits; a commit need not be an entire runnable slice. Begin with workspace/tooling setup, but do not confuse that setup with the end-to-end slice. Shared dependency versions, rustfmt, Clippy, a committed application Cargo.lock, and build checks form the proposed baseline. Exact toolchain/lint/CI settings remain to be chosen during setup; don't enable every pedantic lint by default.

Start with zero authored test code. No permanent test scaffolding or speculative suites. Add tests only for encountered flaky behavior with explicit agreement on the coverage. Use builds, formatting, Clippy, manual real workflows, and temporary dependency probes as evidence. Count production code and tests separately and report the total authored code too.

## Ownership and synchronization

```text
Client input -> HTTP command -> server
Process output -> server-owned Ghostty -> shared screen state
Structural state / terminal screen -> full snapshot -> compressed textual SSE
  -> client state / terminal view -> Ratatui
```

Server-owned live resources (PTYs, child handles, Ghostty objects, tasks and connections) are separate from serializable shared state. Serialize descriptions and screen contents, not resource handles. Terminal screens are outside structural `SessionState`. Runtime-owner drop initiates cleanup; asynchronous reaping, task joins and shutdown guarantees remain to design. Dropping serialized metadata alone does not clean up processes.

Clients do not optimistically alter shared state upon sending a command. Input goes through the PTY; resulting program output drives terminal screen changes. Server output can arrive without any request.

Clients own presentation state such as menus, selection, local focus and viewing position. Focus can be reported as presence for other clients to highlight, without making it global focus. Concurrent terminal input is allowed; do not import Herdr's special single-writer takeover policy.

For the later multi-client behavior, tab size is the minimum available width and height among clients currently viewing that tab. Clients viewing other tabs do not constrain it. No-viewer size behavior remains open and is outside this one-client checkpoint.

## Transport and lifecycle

Use HTTP requests/responses for direct outcomes and compressed textual SSE full snapshots for server-published structural and terminal updates. Requests can trigger events; responses and events have separate purposes. No patch pipeline or replay protocol is adopted. Axum/Reqwest are used by the existing health-exchange scaffolding; compressed snapshot streaming is not integrated into Ship. Synthetic loopback evidence is not a completed Ship terminal integration.

Session revisions reject stale/out-of-order snapshots, not missed attachment updates. Coordinate the current snapshot with subscription to avoid gaps. Exact revision scope and server restart baseline identity/reset remain open. The proposed attachment response carries a server-assigned client/attachment ID and latest state; subsequent header use and reconnect lifetime remain open. Creation responses returning IDs are a recommendation, not an accepted contract.

Coalesce superseded pending states and eventually flush the latest state when capacity returns. Do not queue every intermediate frame or silently drop the final update. Zstd is the leading compression candidate; gzip fallback is only a recommendation. Tower's default predicate excludes SSE and needs explicit adjustment; Reqwest automatic zstd decoding has not been verified. See the transport handoff for bounded live codec evidence.

Explicit server/client modes come first; automatic background server startup is deferred. The server keeps processes running when a client disconnects. Closing a terminal, disconnecting a client, and stopping the server are distinct operations.

Bind to loopback by default. Port customization and configuration are deferred. Local authentication/token checks are explicitly deferred: other local processes/users may reach this endpoint and operate terminals. No non-loopback exposure is part of this slice.

Later remote access can use SSH port forwarding with the same HTTP/SSE protocol. SSH is optional connection setup, not a second set of command handlers. Direct remote exposure would need its own TLS/authentication/access-control policy. Neither path is implemented here.

## Dependency direction

| Area | Starting direction | Evidence/status |
| --- | --- | --- |
| CLI | Clap derives | Used by the implemented scaffold; terminal commands remain to implement |
| Runtime | Tokio | User-selected; blocking PTY reads still need appropriate execution |
| Logging | tracing + tracing-subscriber | Used by the implemented scaffold; keep log output out of UI |
| Errors | Adapt a reference shared AppError/Result/context/macros pattern | Read the reference error module; omit unrelated HTTP/Rig conversions; actor-specific errors need review |
| UI/input | Ratatui/Crossterm | Starting stack for trial |
| Terminal emulation | libghostty-vt | Rust bindings inspected; unstable interface and thread-affinity constraints |
| Session integration | Supplied ratatui-ghostty session with demonstrated compatibility patch | macOS build, 80 enabled upstream tests, user-run example and real-shell capture passed; patch delivery still open |
| PTY | portable-pty candidate with feasibility evidence | 0.9.0 exercised in the example and scratch probe; actual Ship integration remains to implement |
| Actors/bus | Kameo and historical RelayBus/Sink direction | No Ship integration; initial ownership topology remains a recommendation |
| State delivery | Compressed textual SSE full snapshots | Earlier structdiff/JSON Patch results are historical evidence, not the current pipeline |

The existing scaffold uses Utoipa for its health endpoint. Further API documentation and non-Rust SDK work are not required by the terminal checkpoint. Shared Rust types live in core. The earlier structdiff attribute-forwarding work is paused; snapshots do not require generated patch types.

## Accepted screen capture direction

Use the supplied session as the first-slice starting direction, with the small compatibility patch demonstrated in [terminal feasibility evidence](../research/terminal-feasibility.md). This establishes viability, not complete terminal compatibility. It uses a blocking PTY-reader thread and a terminal-processing thread that owns Ghostty, accepts commands, and renders a Ratatui buffer. This can coexist with Tokio; don't run blocking reads on Tokio workers.

Ratatui's buffer/cell types already support Serde with its serde feature. Use owned screen/cell types with conversions to/from Ratatui so the shared schema and derives are under Ship's control. This conversion maps already-interpreted display data; it is not a new terminal emulator or diff engine.

The probe preserved accessible text/color/modifier/underline/skip/diff-option fields and cursor data, and exercised styled Unicode, output scrolling and resize. It reused Ratatui color/modifier types in scratch cells; the final owned schema is not selected. Nondefault skip/diff options, alternate cursor variants and all reset semantics were not exhaustively exercised. Ratatui equality normalizes unset symbols to spaces.

The capture path must not share live session handles across processes. The supplied session exposes size, buffer blitting and cursor state, but under separate locks. Quiescent/stable captures passed; coherent publication under continuous output remains an implementation-contract concern, not a completed atomic-capture guarantee.

Known source-level concerns in the supplied session: some write/resize errors are discarded; read errors resemble EOF; dropping the handle signals shutdown but doesn't prove reader interruption, child termination/reaping, or joined threads. Ship must retain and manage child lifecycle. These are checks, not demonstrated runtime failures or grounds for an automatic rewrite.

## Historical diff evidence and current publication direction

The earlier structdiff default whole-vector replacement and JSON Patch experiments passed captured-state equality checks. They are retained as evidence, not instructions to implement a patch pipeline. Subsequent compression and live SSE results support full snapshots; see the [transport handoff](../research/terminal-transport-handoff.md).

Profile the runnable workflow before adding a custom screen algorithm or changing representation. Changed-only publication, bounded cadence and latest-state coalescing remain necessary; exact mechanics and cadence are unchosen. No further collection experiment is a prerequisite to the proposed metadata checkpoint.

## First-slice command concepts

Final HTTP routes, CLI names and payloads remain open. Herdr is a behavioral reference, not a command catalog to copy.

- Connect/register client and obtain coordinated current state.
- Create/open the initial terminal with an explicit identity; exact startup ownership and HTTP creation response remain to specify.
- Send terminal input to that identity, preserving distinctions between keys, committed text and paste.
- Report client view dimensions; distinguish absolute geometry from future split-ratio adjustments.
- Close a terminal, disconnect a client, and stop the server as distinct lifecycle actions.

Herdr has `api snapshot`, `pane send-text`, `pane send-keys`, and `pane run` (text plus Enter). Its CLI `pane resize` nudges a split ratio; interactive absolute dimensions are connection messages. Its special terminal attachment writer/takeover policy is not Ship's chosen concurrent-input policy.

## Evidence and limits

- Published ratatui-ghostty 0.2.0 source was counted: approximately 511 nonblank/non-comment-only lines for widget/style/input helpers, 664 for its session wrapper, 236 for host-color queries; tests/examples excluded. These are source measurements, not guaranteed savings.
- Published wrapper 0.2.0's binding dependency 0.1.1 failed with Zig 0.16.0 because its pinned Ghostty requires 0.15.2. A scratch wrapper dependency/API update to bindings 0.2.1 at 8953a740bc378cec3e07e1f6ca949f0595eab19b built with Zig 0.16.0. Patch delivery remains open; no Ship vendoring/fork has been created.
- All 80 enabled upstream wrapper tests passed after adapting three test constructors without changing assertions. One ignored callback-movement reproducer was not run. Cyan confirmed the multiplexer example worked interactively.
- Parent-rerun scratch probes verified real-shell screen reconstruction, cursor-field preservation, JSON patch application, unchanged empty diffs, and graceful shell reaping/reader destruction/session stop. All 18 collection strategy/sample combinations passed equality. Those native capture probes did not test HTTP/SSE or Linux. Later synthetic HTTP/SSE evidence is in the transport handoff. Details and limitations are in [terminal feasibility evidence](../research/terminal-feasibility.md).
- Inspected locally installed Ratatui buffer/cell definitions for Serde and field representation.
- Reran `/tmp/diffprobe` offline against structdiff 0.7.3. Four changes, nested recursion, a map modification and enum replacement survived binary serialization/deserialization and automatic application; unchanged values yielded no diffs. Toy payload was 176 JSON bytes / 125 bincode bytes. This is not a screen benchmark or a choice of binary transport.
- Herdr commands/remote behavior inspected at `d6b40d4edd550ccea081f089605a64314f8c8b27`. Herdr uses local sockets bridged through SSH command stdin/stdout, unlike the proposed HTTP-over-SSH-port-forwarding route for Ship.
- No production code, dependencies, automated tests, or public interfaces have been implemented for Ship.

Primary references:

- [ratatui-ghostty published archive](https://crates.io/api/v1/crates/ratatui-ghostty/0.2.0/download)
- [Session implementation](https://codeberg.org/jint/ratatui-ghostty/src/branch/main/src/session.rs)
- [Single-terminal example](https://codeberg.org/jint/ratatui-ghostty/src/branch/main/examples/single_terminal.rs)
- [Ghostty Rust bindings](https://docs.rs/libghostty-vt/latest/libghostty_vt/index.html)
- [structdiff](https://github.com/knickish/structdiff)
- [Herdr CLI declarations](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/src/cli/spec.rs)
- [Herdr client protocol](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/src/protocol/wire.rs)
- [Herdr remote bridge](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/src/remote/attach.rs)
- [Zellij per-client tab sizes](https://zellij.dev/news/nested-sessions-kitty-graphics-new-ui/)

Temporary research files under `/tmp` are not durable project artifacts. Their verified results are summarized here; they may need recreating.

## Remaining decisions

The supplied capture approach remains the terminal integration direction; the former initial diff strategy is superseded by compressed snapshots. The [session design](session-structure.md) is the current record for structure, IDs and actors. Proposed metadata scope, exact request/response shapes, attachment coordination, revision/restart semantics, actor topology, coherent screen publication and terminal shutdown guarantees must not be inferred during implementation. Delivery of the demonstrated wrapper patch is also open. Configuration/plugins remain later work.
