# First terminal slice

Build one real interactive terminal across Ship's separate client/server processes. This records the agreed starting direction, not a completed implementation or a frozen product specification. The broader [product brief](../../product-brief.md) remains revisable.

The [Wayfinder map](../../.scratch/first-terminal-slice/map.md) indexes decision records and outstanding questions. It ends at an implementation-ready first-slice plan, not a completed application.

## Observable checkpoint

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
Shared state -> structdiff patch -> SSE -> client state copy -> Ratatui
```

Server-owned live resources (PTYs, child handles, Ghostty objects, tasks and connections) are separate from serializable shared state. Serialize descriptions and screen contents, not resource handles.

Clients do not optimistically alter shared state upon sending a command. Input goes through the PTY; resulting program output drives terminal screen changes. Server output can arrive without any request.

Clients own presentation state such as menus, selection, local focus and viewing position. Focus can be reported as presence for other clients to highlight, without making it global focus. Concurrent terminal input is allowed; do not import Herdr's special single-writer takeover policy.

For the later multi-client behavior, tab size is the minimum available width and height among clients currently viewing that tab. Clients viewing other tabs do not constrain it. No-viewer size behavior remains open and is outside this one-client checkpoint.

## Transport and lifecycle

Use HTTP requests/responses for direct outcomes and SSE for server-published updates, initially with JSON patches. Requests can trigger events; responses and events have separate purposes. Axum/Reqwest are suggestions pending dependency validation, not a completed integration.

Full snapshots are available for attachment and recovery. Updates carry a server-instance identity, base revision and new revision. A client ignores covered updates, applies a patch only against its expected baseline, and requests a fresh snapshot when continuity is lost. Coordinate snapshot creation with stream subscription to avoid gaps; exact mechanism is an open decision.

Explicit server/client modes come first; automatic background server startup is deferred. The server keeps processes running when a client disconnects. Closing a terminal, disconnecting a client, and stopping the server are distinct operations.

Bind to loopback by default. Port customization and configuration are deferred. Local authentication/token checks are explicitly deferred: other local processes/users may reach this endpoint and operate terminals. No non-loopback exposure is part of this slice.

Later remote access can use SSH port forwarding with the same HTTP/SSE protocol. SSH is optional connection setup, not a second set of command handlers. Direct remote exposure would need its own TLS/authentication/access-control policy. Neither path is implemented here.

## Dependency direction

| Area | Starting direction | Evidence/status |
| --- | --- | --- |
| CLI | Clap derives | User-selected; no integration yet |
| Runtime | Tokio | User-selected; blocking PTY reads still need appropriate execution |
| Logging | tracing + tracing-subscriber | User-selected; keep log output out of UI |
| Errors | Adapt the reference's shared AppError/Result/context/macros pattern | Read the reference error module; omit unrelated HTTP/Rig/Kameo conversions |
| UI/input | Ratatui/Crossterm | Starting stack for trial |
| Terminal emulation | libghostty-vt | Rust bindings inspected; unstable interface and thread-affinity constraints |
| Session integration | Supplied ratatui-ghostty session with demonstrated compatibility patch | macOS build, 80 enabled upstream tests, user-run example and real-shell capture passed; patch delivery still open |
| PTY | portable-pty candidate with feasibility evidence | 0.9.0 exercised in the example and scratch probe; actual Ship integration remains to implement |
| State diff/apply | structdiff 0.7.3 with serde; default whole-vector replacement | JSON screen patch/application verified; optimization explicitly deferred until profiling real usage |

Defer OpenAPI/Utoipa and non-Rust SDKs. Shared Rust types live in core. If generated patch types later need custom derives, evaluate an upstream change/small fork then, not preemptively. structdiff 0.7.3 provides fixed feature-controlled derives, not arbitrary attribute forwarding.

## Accepted screen capture direction

Use the supplied session as the first-slice starting direction, with the small compatibility patch demonstrated in [terminal feasibility evidence](../research/terminal-feasibility.md). This establishes viability, not complete terminal compatibility. It uses a blocking PTY-reader thread and a terminal-processing thread that owns Ghostty, accepts commands, and renders a Ratatui buffer. This can coexist with Tokio; don't run blocking reads on Tokio workers.

Ratatui's buffer/cell types already support Serde with its serde feature. Use owned screen/cell types with conversions to/from Ratatui so the shared schema and derives are under Ship's control. This conversion maps already-interpreted display data; it is not a new terminal emulator or diff engine.

The probe preserved accessible text/color/modifier/underline/skip/diff-option fields and cursor data, and exercised styled Unicode, output scrolling and resize. It reused Ratatui color/modifier types in scratch cells; the final owned schema is not selected. Nondefault skip/diff options, alternate cursor variants and all reset semantics were not exhaustively exercised. Ratatui equality normalizes unset symbols to spaces.

The capture path must not share live session handles across processes. The supplied session exposes size, buffer blitting and cursor state, but under separate locks. Quiescent/stable captures passed; coherent publication under continuous output remains an implementation-contract concern, not a completed atomic-capture guarantee.

Known source-level concerns in the supplied session: some write/resize errors are discarded; read errors resemble EOF; dropping the handle signals shutdown but doesn't prove reader interruption, child termination/reaping, or joined threads. Ship must retain and manage child lifecycle. These are checks, not demonstrated runtime failures or grounds for an automatic rewrite.

## Initial diff strategy and optimization policy

Start with structdiff's default replacement of a changed cell vector. JSON generation/deserialization/application passed for captured screens, including resize and unchanged state. The finer collection experiments demonstrated tradeoffs, not a production bottleneck.

Build the runnable slice, profile actual behavior, then benchmark or change only a demonstrated bottleneck. No further collection experiments, custom algorithm, release benchmark or row-based representation is a prerequisite now. All collected timing samples were debug-build diff generation only; release may be faster, but does not shrink identical JSON payloads.

A custom screen StructDiff implementation remains an escape hatch, not approved work. Optimization can preserve server ownership and HTTP/SSE while changing patch schemas; coordinate client compatibility if that happens. See the [diff decision](../../.scratch/first-terminal-slice/issues/03-screen-diffs.md).

## First-slice command concepts

Final HTTP routes, CLI names and payloads remain open. Herdr is a behavioral reference, not a command catalog to copy.

- Connect/register client and obtain coordinated current state.
- Create/open the initial terminal and return an explicit identity; exact startup ownership remains to specify.
- Send terminal input to that identity, preserving distinctions between keys, committed text and paste.
- Report client view dimensions; distinguish absolute geometry from future split-ratio adjustments.
- Close a terminal, disconnect a client, and stop the server as distinct lifecycle actions.

Herdr has `api snapshot`, `pane send-text`, `pane send-keys`, and `pane run` (text plus Enter). Its CLI `pane resize` nudges a split ratio; interactive absolute dimensions are connection messages. Its special terminal attachment writer/takeover policy is not Ship's chosen concurrent-input policy.

## Evidence and limits

- Published ratatui-ghostty 0.2.0 source was counted: approximately 511 nonblank/non-comment-only lines for widget/style/input helpers, 664 for its session wrapper, 236 for host-color queries; tests/examples excluded. These are source measurements, not guaranteed savings.
- Published wrapper 0.2.0's binding dependency 0.1.1 failed with Zig 0.16.0 because its pinned Ghostty requires 0.15.2. A scratch wrapper dependency/API update to bindings 0.2.1 at 8953a740bc378cec3e07e1f6ca949f0595eab19b built with Zig 0.16.0. Patch delivery remains open; no Ship vendoring/fork has been created.
- All 80 enabled upstream wrapper tests passed after adapting three test constructors without changing assertions. One ignored callback-movement reproducer was not run. Cyan confirmed the multiplexer example worked interactively.
- Parent-rerun scratch probes verified real-shell screen reconstruction, cursor-field preservation, JSON patch application, unchanged empty diffs, and graceful shell reaping/reader destruction/session stop. All 18 collection strategy/sample combinations passed equality. No HTTP/SSE or Linux checks were performed. Details and limitations are in [terminal feasibility evidence](../research/terminal-feasibility.md).
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

The capture approach and initial diff strategy have been reviewed and accepted; those feasibility checks no longer block implementation planning. See open children of the [Wayfinder map](../../.scratch/first-terminal-slice/map.md) for the remaining exact command/attachment contract and delivery of the demonstrated wrapper patch. Request shapes, SSE framing/snapshot coordination, coherent capture publication, binding names and terminal creation/shutdown behavior must not be silently inferred during implementation. Configuration/plugins belong to a later slice, not this map's frontier.
