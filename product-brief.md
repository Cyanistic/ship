# Ship: product brief and roadmap

Ship is a compact Rust terminal workspace for coding-agent work. It keeps shells, editors and agents together, lets work continue after the UI closes, shows which agents need attention, and gives agents a small automation interface.

The aim is to recover most of the daily-use value of Herdr with substantially less owned code and maintenance burden. Ship is a side project, not a feature-for-feature rewrite or an upstream refactor. The name treats agents as crewmates and plays on shipping software. The intended executable is `ship`; package and distribution identifiers remain to be checked.

This brief records product direction and a phased roadmap. It is not an implementation specification. Milestones describe proposed work, not completed capabilities, and have no calendar commitments. Scope remains revisable as working slices provide evidence.

Build through fine-grained, coherent commits. A commit need not be a complete vertical slice; a vertical slice demonstrates a narrow behavior end to end, and a milestone is a checkpoint for judging working behavior and choosing what comes next. See [the first terminal slice](docs/planning/first-terminal-slice.md) and its [Wayfinder decision map](.scratch/first-terminal-slice/map.md).

## Direction

### Own the domain, rent the mechanisms

Use existing libraries for solved capabilities. Ship should own workspace behavior, session rules and the composition of those capabilities, rather than bespoke implementations of serialization, terminal emulation, playback, clipboard access or transport.

Use idiomatic Rust types and abstractions to express invariants once. Use derives and macros when they remove independently maintained copies of the same information. Do not pursue fewer lines through opaque compression or a generic framework built for hypothetical features.

The starting stack is Rust with Clap, Tokio, tracing/tracing-subscriber, and shared error handling adapted from the reference's error module. Trial Ratatui/Crossterm, `libghostty-vt`, the supplied `ratatui-ghostty` session, and `structdiff` before replacing their mechanics with owned code. `portable-pty` passed the scratch probe and remains the candidate for PTY integration; actual Ship integration and Linux validation remain. Rodio and Copypasta remain later candidates, not selections.

Ratatui draws the workspace; Ghostty interprets programs' terminal output and maintains screen state on the server. The intended reuse path converts the supplied session's rendered Ratatui cells into owned, serializable screen/cell types, then converts those types back for client rendering. The macOS scratch probe verified captured screen conversion and JSON patch application, and Cyan confirmed the patched wrapper's multiplexer example worked. Use default whole-vector diffs initially; optimize only after profiling the actual runnable slice reveals a bottleneck. This is feasibility evidence, not complete platform/rendering coverage. See [terminal evidence](docs/research/terminal-feasibility.md).

### Size is a design constraint, not the acceptance test

Aim for roughly 20,000 lines of authored production Rust while retaining most of the functionality Cyan actually uses. That budget is a hypothesis to test, not a proven estimate or a reason to omit necessary behavior.

Report production code, tests, generated code and dependency code separately. Also report the full authored total including tests, so reductions are not hidden by moving code between categories. Working behavior, understandable boundaries and maintenance burden matter more than hitting an exact number.

Start with zero authored test code and no speculative test scaffolding. Add tests only after encountering flaky behavior and agreeing that a specific test is warranted. Verify increments with builds, formatting, Clippy, and real workflows; temporary experiments may establish dependency behavior without adding permanent tests. Omitting tests reduces the authored total, not the separately reported production-code count.

### Platform policy

Linux is Cyan's primary environment. Linux and macOS are priority platforms; Windows and other platforms are best effort.

Choose cross-platform dependencies by default. Validate priority platforms deliberately and record other platform gaps without making them initial-release blockers. Do not write speculative platform-specific subsystems in advance.

### Local first, remote-compatible boundary

The initial product is local. Remote attachment is not required for the first daily-driver release.

Keep a clear client/server boundary that can later cross a network: explicit commands, responses and events; shared state distinct from client presentation state; live process and PTY resources private to the server. The client should not depend on direct access to server internals.

The initial architecture uses separate client and server processes from one executable, with HTTP request/response and SSE for server events. Bind to loopback by default; port customization is deferred. The server owns authoritative shared state. Clients apply full snapshots and revisioned JSON patches generated by `structdiff`, while keeping presentation choices local. Snapshot/subscription coordination must prevent attachment gaps; a server-instance identity distinguishes revisions across restarts.

SSH port forwarding is the intended later remote connection option, not part of the application protocol. Direct remote HTTP access would require a separate authentication/encryption/access-control design. Do not build remote infrastructure for the first slice.

Local authentication is explicitly deferred. Loopback does not restrict access to the current user: other local processes or users may reach the control endpoint. Do not describe this initial endpoint as authenticated or enable non-loopback exposure without revisiting security.

Use a Cargo workspace with `ship` (binary, CLI and startup), `ship-client`, `ship-server`, and `ship-core` (genuinely shared types, traits and helpers). Client and server depend on core rather than each other's internals. The first implementation checkpoint is narrower than roadmap milestone 1: one interactive terminal across the real client/server path.

## Initial daily-driver scope

The first daily-driver release comprises milestones 1 through 4: a local terminal workspace, persistent sessions, agent awareness and integrations, and automation. Each milestone should leave a usable increment. The order expresses capability goals, not a requirement to postpone foundational API boundaries until milestone 4.

### 1. A terminal workspace to work in

Reach this capability milestone through smaller runnable checkpoints, starting with [one real terminal across the client/server path](docs/planning/first-terminal-slice.md). The list below is not a single implementation batch.

Include:

- Workspaces, tabs and split panes.
- Focus, resize, zoom, move, rename and close operations.
- Configurable keyboard bindings.
- Mouse focus, text selection and pane resizing.
- Scrollback, copy/paste and terminal text search.
- Terminal emulation and PTY handling through suitable libraries.

Completion evidence:

- Pi, Neovim, ordinary shells and running tests work side by side.
- Repeated splitting and resizing preserve usable rendering and input.
- Alternate-screen applications enter and exit correctly.
- Selection, copying, pasting and search work in representative sessions.
- These behaviors are exercised on Linux and macOS, with known limitations recorded.

### 2. Leave and come back

Include:

- A background server that owns terminals and processes independently of the UI.
- Detach and reattach without interrupting running work.
- Save and restore workspace/tab/pane identities, layout and working directories.
- Supported agent conversation resume after restart.
- Clear recovery behavior when a directory or resume target is unavailable.

Keep the promises distinct:

| Situation | Expected behavior |
| --- | --- |
| The UI closes while the server remains alive | Existing processes continue; the client can reconnect. |
| The server or machine restarts | Restore saved structure and recreate supported sessions. Original processes do not survive. |
| An agent supports conversation resume | Use its saved session identity and supported resume mechanism. |

Persist reconstructible session descriptions, not process handles, PTYs or connections. Safe saves and a policy for saved-format changes belong here. Exact screen-history replay is a follow-on capability, not a prerequisite for basic persistence.

Completion evidence:

- Closing and reopening the client preserves running work.
- Restarting the server reconstructs the saved structure and distinguishes resumed work from unavailable work.
- An interrupted save does not destroy the last usable saved state.
- Representative missing-directory and unsupported-resume cases produce understandable results.

### 3. Know which agent needs attention

Include:

- Working, blocked, idle and unknown agent states.
- An unseen-completion indicator distinct from an agent being actively busy.
- An agent list and jump-to-agent action.
- Desktop and audio notifications with simple configuration.
- A shared reporting interface for agent hooks.
- Pi integration first, then the agents Cyan actually uses.
- Screen-based detection where explicit reporting is unavailable and the behavior can be validated.

An integration should be a small adapter to shared behavior, not require a plugin framework. Prefer explicit reports where available; do not make a broad catalog of screen heuristics an initial deliverable.

Completion evidence:

- Several concurrent agents can be distinguished by state and location.
- An agent requesting input or finishing unseen work is discoverable without visiting every pane.
- Unknown state is shown honestly rather than treated as completion.
- Notifications respect configuration, and unavailable optional notification/audio facilities do not break terminal work.
- Pi lifecycle reporting is demonstrated against real sessions.

### 4. Let agents operate the workspace

Include a CLI and machine-readable control interface for:

- Listing and inspecting workspaces, tabs, panes and agents.
- Creating, splitting, focusing and closing panes.
- Running commands and sending text or logical keys.
- Reading visible output and recent scrollback.
- Prompting a named agent.
- Waiting for output or an agent-state transition.
- Receiving state/event updates.

Use explicit target identities and useful errors. Automation must not accidentally act on whichever pane the user happens to focus. Share operation definitions with the application rather than reimplementing behavior in CLI handlers.

Completion evidence:

- An agent can start a sibling task, submit work, wait for a result and read it through the control interface.
- The sequence uses returned identities rather than guessed identifiers.
- User focus changes do not redirect targeted automation.
- Invalid targets, closed panes, timeouts and unavailable output are reported clearly.

## Later roadmap

### 5. Remote attachment

Design remote semantics before selecting a transport. Add attachment to a remote server, connection-loss handling and reconnect behavior. Preserve the local domain/control boundary rather than creating a parallel remote implementation.

Completion evidence: a dropped connection does not stop remote work, and reconnection restores a coherent view without silently duplicating commands or losing track of their outcomes. Exact delivery/retry guarantees remain a design question.

### 6. Multiple clients and machines

Add independent client views, saved machine connections and a combined local/remote workspace interface. Shared session state and per-client view state must remain distinct.

Completion evidence: clients do not fight over their local focus or corrupt shared session state; one unavailable machine does not make unrelated local work unusable.

## Deferred supporting features

| Capability | Initial position |
| --- | --- |
| Configuration system | Defer past the first terminal slice; fixed defaults and minimal operational bindings first. Storage, key matching, reload and shared/per-client scope remain open for a customization slice. |
| Plugin framework and marketplace | Defer until a concrete extension need exists; no first-slice runtime, manifests or registry. Herdr/Zellij ownership research is context, not an adopted plugin model. |
| Built-in worktree management | Use Git or Worktrunk in ordinary panes. |
| Self-update and release channels | Use normal installation/distribution tooling initially. |
| Every agent integration | Expand from actual use through the shared reporting interface. |
| Advanced terminal graphics and inline images | Separate compatibility work; not an initial requirement. |
| Exact saved-screen replay | Follow basic persistence if useful. |
| Live server upgrades and process handoff | Defer; distinguish ordinary restart/restore from process survival. |
| Elaborate themes and notification rules | Start with a small configuration surface. |
| Full Windows/platform parity | Best effort outside Linux and macOS. |

## Decisions still open

Resolve these during slice design and dependency experiments, not by silently treating this brief as a specification. Previously discussed choices and remaining first-slice questions are indexed in [the Wayfinder map](.scratch/first-terminal-slice/map.md).

- Delivery of the demonstrated ratatui-ghostty compatibility patch (pinned fork versus source/subtree import); no method has been selected or installed in Ship. Supplied session plus portable-pty passed macOS feasibility checks; actual application integration, coherent publication and Linux coverage remain.
- Exact HTTP operations, SSE subscription/snapshot lifecycle, message shapes, and restart identity behavior. Axum/Reqwest are suggested HTTP libraries, not yet validated dependencies.
- Final screen/cell schema and detailed rendering coverage. Default whole-vector structdiff is selected to start; performance optimization is deferred, not a first-slice decision blocker.
- Detailed shared-state fields and per-client view state. Focus is client-local but reportable as presence; concurrent terminal input is allowed. Tab geometry follows the minimum available width and height among clients currently viewing that tab; behavior with no viewers remains to be specified.
- Saved-state format evolution, recovery policy and agent resume descriptions.
- Agent reporting protocol and the next integrations after Pi.
- Minimal fixed interaction/exit bindings for the first slice. Configuration format, storage, configurable key matching/reload and plugin ownership are deferred to later slices; see [ownership research](docs/research/config-plugin-ownership.md).
- Remote authentication, command delivery/retry behavior and transport, before remote implementation.
- Repository location, project license and package/distribution identifiers.

## Reference and scope of evidence

Herdr is the behavioral reference, not the structural template. The initial inventory examined checkout `d6b40d4edd550ccea081f089605a64314f8c8b27` (version 0.9.3). Useful references in this notebook:

- `repos/herdr/repo/README.md`: product behavior and persistence distinctions.
- `repos/herdr/repo/docs/next/website/src/content/docs/cli-reference.mdx`: user-facing command families.
- `repos/herdr/repo/src/cli/spec.rs`: declared control surface.
- `repos/herdr/repo/src/sound.rs`: an inspected example of implementation responsibility Ship intends to rent from a library.

The inventory used docs, declarations and limited code checks, not a complete behavioral audit. No implementation-size estimate has been demonstrated by a prototype. The roadmap reflects Cyan's starting priorities. The first-slice ownership and transport direction is recorded, and macOS dependency feasibility has been demonstrated. Production client/server implementation, Linux validation and full behavior coverage remain future work.
