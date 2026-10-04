# Design

## Context

See [proposal.md](proposal.md) for motivation and capability scope. The three specs define observable behavior. [design/program.md](design/program.md) supplies the detailed file map and verification exercises; active sections of the product/architecture papers supply the broader constraints.

Cyan's latest program decisions take precedence over older snippets: bare `ship` is the client workflow, unsafe is permitted, dependencies use ordinary compatible constraints, HealthResponse is plain data, and README addresses end users. Superseded details sections are historical. Cyan supplied the reference error module to resolve the previously unknown err! syntax; this artifact records that resolution without modifying the papers.

Read-only inspection found an incomplete root Cargo.toml, four crate manifests and core lib/protocol files. The root still contains Utoipa 5, Progenitor, exact Reqwest pins, an unsafe denial and unused broader dependencies. Core lib references error.rs, which does not exist. No accepted build or lockfile evidence exists. These are abandoned remnants to reconcile under a later implementation request, not working behavior. The terminal experiment is independent and must not supply production patches or dependencies.

## Goals / Non-Goals

**Goals:**
- Keep four shallow ownership boundaries: app composition/startup, shared data/errors, client HTTP policy/methods, server listener/schema.
- Deliver independently runnable checkpoints with real output and evidence before progression.
- Retain direct shared-type identity and a small endpoint-authoring surface without generation infrastructure.

**Non-Goals:**
- No permanent authored tests, CI/deployment setup, terminals, SSE, compression or management API.
- No cross-platform parity claim beyond the exercised environment.
- No client build script, duplicate generated DTOs, AI invocation, generator fork, schema downgrade or Java tooling.
- No speculative configuration framework, second error framework or expanded server trust boundary.

## Decisions

### 1. Foundation slice: checked executable and shared errors

Create/reconcile the workspace and four crates. App composes core/client/server; client/server each use core but never each other's implementation at build/runtime. Keep minimum real modules, not future-feature panic stubs. Preserve program.md's profile choices and restrained Clippy checks, but remove the unsafe deny rule. Centralize compatible dependency requirements, including Utoipa `6.0` and Reqwest `0.13.4` with defaults disabled and json/rustls. Cargo owns lockfile resolution.

This is smaller than importing the experiment or keeping abandoned generator/framework dependencies. Unsafe remains available but the selected launch mechanism needs none.

Build a genuine help/version executable first. Health/server operations enter slice 2. End-user README explains the available checkpoint; internal evidence stays in program.md's Deviation log during authorized implementation.

**Error construction.** Retain the planned contextual AppError/internal-external structure, shared categories, optional data/cause and lossy foreign-error strings. ResultExt uses `context`, `error`, `warn`, `debug`, `trace`; reporting returns the unchanged result with caller location.

Use the public err! grammar observed in the reference, adapted to Ship constructors and its category names:

```rust
err!(Configuration, "invalid setting")
err!(Network, "cannot reach {}", target)
err!(Configuration, "invalid target", @data: { target: safe_target })
err!(Internal, "startup failed", @source: app_error)
err!(Io, "log creation failed", @external: io_error)
err!(@external Serialization, serde_error)
err!(@external code_expression, foreign_error, @data: data)
```

Code arguments accept either variants in the macro's hygienically imported ErrorCode namespace or explicit expressions. Messages accept literal/expression or formatting arguments. Message forms allow chained `@data:`, `@source:` and `@external:` modifiers. Data accepts a serializable expression or object shorthand; follow the reference's behavior of omitting data when conversion to JSON fails. Source attaches an AppError cause; external attaches a foreign-error wrapper using the outer category. Pure @external creates ExternalError without an extra message. Macro expansion returns an AppError value, not Result and not an early return.

Adapt helpers to Ship's chosen representation instead of copying the reference wholesale. Foreign errors become diagnostic strings, not color-eyre reports. Preserve cause/category behavior and hygienic `$crate`/serde_json references so callers need no direct serde_json import for shorthand. Do not adopt the reference's actor/provider adapters, message-based category guessing or HTTP response implementation. Macro rules must be compiled and exercised during slice 1; source inspection is not proof of expansion correctness. Convenience macros beyond err! are not required to finish this checkpoint.

**Observable checkpoint:** local formatting/Clippy/debug-release builds and help/version work without a server. Inspect serialization/diagnostics and exercise macro forms through disposable compilation, without adding a permanent test suite.

### 2. Health slice: real requests, explicit targets and bounded startup

Wire the root parser to optional server subcommand plus root --server-url. Reject combining them; validate the illustrative Clap declaration in program.md rather than relying on uncompiled syntax. Use:

```sh
ship
ship --server-url http://127.0.0.1:43180
ship server
ship server --port 43180
```

Omitted target selects automatic default-local startup; any explicit target is connect-only. One binary avoids external launcher/daemon tools. No client subcommand or server stop/status API is added. Print one health JSON line on success, diagnostic context on stderr, operational failure exit 1 and Clap usage exit 2.

**Shared data.** Keep route/default/protocol constants in core protocol.rs. HealthResponse has service, protocol_version, version and Serde derives, with no constructor/validation methods. Server constructs service ship, protocol version 1 and informational package version. Client-owned health logic validates compatibility. This keeps shared DTOs free of server/client behavior while avoiding duplicate types.

**Client.** target.rs resolves root-relative health and redacts userinfo in Display/Debug/errors. transport.rs owns the Reqwest client, no-redirect policy, exact status checks, bounded request/body deserialization and explicit error classification. api.rs is one private ordinary source file containing thin endpoint methods over core DTOs. health.rs exposes the stable facade and default-local probe. No method patch may silently change transport/startup policy.

Shared source alone does not prevent wire drift; the actual request is the evidence. A handwritten/agent-authored endpoint is chosen over Progenitor or duplicate generated types for this checkpoint, not as a permanent rejection of generation.

**Startup.** local.rs implements probe → launch if refused → wait for compatible health → return. TCP refusal establishes absence; timeout, bad JSON/status/identity or other failures do not. Resolve current_exe and directly spawn server with hidden --background-child. Redirect stdin to null and stdout/stderr to cloned handles for a unique mode-0600 temporary log. In synchronous child entry, safe setsid runs before runtime/tracing/listening. No shell, pre_exec, manual fork or preassigned child process group.

Keep only private launch-attempt state: child handle and log path. Connect bound is one second; complete health bound two seconds. Poll within a five-second absolute readiness deadline at 100 ms intervals, capping attempts by remaining time. Inspect child exit and make a final bounded probe for a concurrent winner. Cleanup terminates/reaps only this invocation's failed child; successful daemon is left running. Never terminate an existing listener. Keep logs, report PID/path, and disclose the attempted directory if creating a log fails.

**Server.** Axum serves loopback only; reject non-loopback serve arguments and port zero. Structured bind errors distinguish conflict. App initializes tracing once; tower-http logs method/path/status/duration at the default info filter without arbitrary bodies/headers. SIGINT/SIGTERM drain and exit within five seconds.

**Observable checkpoint:** real health, launch/reuse, daemon survival after client/terminal exit, explicit/custom-port serving, explicit-target failures without fallback, request traces, private logs, bounded launch failure/cleanup, concurrent convergence and graceful shutdown. Record only exercised platform behavior.

### 3. Schema/update slice: inspect and reconcile without generation

Add Utoipa 6 ToSchema metadata to core DTOs and route descriptions to server health. server/schema.rs constructs an OpenAPI 3.1 document through server's pure openapi() function. No listener, tracing, tasks or runtime is needed, and client gains no server dependency. Keep operation ID health and HTTP 200 metadata. Inspect literal route attributes against constants if the macro does not accept const expressions.

This retains schema documentation without relabeling 3.1 as 3.0 or constraining server tooling to a generator. Inspect through a disposable harness outside the production tree; do not ship a schema command, endpoint or persistent tool.

Document the endpoint-update workflow in development planning: inspect corresponding route/DTO/Serde/schema, edit affected methods, review policy boundaries, run checks and request the corresponding rebuilt server. Accepted Rust is source of truth; normal builds neither regenerate nor call models. Do not label api.rs auto-generated.

Exercise a temporary DTO change leaving an incompatible consumer, observe compiler feedback, reconcile and make a real request. Separately change the route while keeping the old client path deliberately, observe failure despite compilation, then reconcile and succeed. Restore baseline and recheck. Shared HEALTH_PATH normally removes literal drift; the deliberate mismatch shows the limit of compiler evidence.

**Observable checkpoint:** pure schema output agrees with the actual route/serialization, direct shared DTO use is inspected, both controlled update exercises are observed, baseline is restored and all applicable local checks/health behavior pass.

Keep future generator analysis in [design/architecture.md](design/architecture.md#future-deterministic-generation-options). openapi-to-rust's named replacements/type_mappings remain unproved and optional operation builders can assume owned model constructors/fields. Progenitor's 3.1 migration and older-Utoipa alternative have different costs; external OpenAPI Generator adds provisioning/template obligations. None is selected or an implementation task. Recurring drift or maintenance burden calls for a reviewed decision, not an automatic migration.

## Risks / Trade-offs

- [Agent-authored code can compile while disagreeing with the API] → Review wire details and make requests against the rebuilt server; do not claim freshness automation.
- [Partial files contain superseded choices] → Reconcile only under an implementation request, remove obsolete dependencies/configuration then, and never treat remnants as verified.
- [Process detachment is platform-sensitive] → Exercise terminal closure, session identity, log privacy, deadlines and child cleanup on the actual environment; report unavailable checks.
- [Loopback identity is not authentication] → Preserve the loopback-only server boundary and describe checks as accidental mismatch detection.
- [Concurrent launch can lose the bind race] → Reprobe for a winner, bound each client outcome and verify no idle losing children survive.
- [Compatible ranges can select unexpected versions] → Commit Cargo's resolution and establish compatibility through checks and live behavior, not exact manifest pins.
- [the reference macro source is not drop-in Ship implementation] → Preserve its syntax while adapting storage/helper names, compile representative expansions, and avoid importing unrelated dependencies.

## Migration Plan

There is no accepted production deployment to migrate. In a later explicitly authorized implementation run, reconcile remnants and implement one vertical slice, verify it, record evidence and stop. Do not begin the next slice in the same run. No CI/deployment or production rollout is included.

Temporary update/inspection exercises restore the intended baseline before completion. Preserve unrelated experiments and files throughout. No cleanup, commits, pushes or implementation changes are authorized by creation of these artifacts.
