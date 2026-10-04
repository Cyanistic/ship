# Initial scaffolding

Status: reopened for review. Audience: mixed. Cyan authorized revising the plan to an agent-authored, checked-in client; the revised paper is not yet locked. Architecture and program are reopened under the ripple rule. The previous locked revision is preserved below. Approval does not authorize implementation.

## Goal

Give Ship's developers a buildable local foundation with useful checks, observable behavior, and a minimal working server/client exchange before terminal work begins. Finish the checkpoint with a reviewed, agent-authored Rust client that uses the same shared DTOs as the server, performs a real request, and has an exercised API-update workflow. Normal builds consume committed source without invoking an AI model or client generator.

## User stories

### P1: Develop against a checked foundation

As a Ship developer, I want the project to build and provide reproducible local checks so subsequent slices start from a usable baseline.

- **Given** a checkout and the documented prerequisites, **when** I run the documented local checks, **then** formatting, linting, and build checks complete successfully.
- **Given** the application is built, **when** I request command-line help, **then** I can discover how to start the server and invoke the client.
- **Given** a slice is ready, **when** its verification is recorded, **then** its applicable local checks and manual acceptance scenarios have passed before the next slice begins.

Independent verification: build from the documented prerequisites, run the local checks, and exercise command-line help without needing a running server.

### P2: Exercise an observable local server/client exchange

As a Ship developer, I want to start the server and check it from the client so I can exercise the foundation before schema/update verification or terminal behavior.

- **Given** no default local server is running, **when** I invoke the client without a custom server URL, **then** it starts a local server in the background, connects to it, and reports a successful health response.
- **Given** the default local server is already running, **when** I invoke the client without a custom server URL, **then** it uses that server rather than starting another one.
- **Given** the client started the local server, **when** the client exits, **then** the server remains running and a subsequent client invocation can reach it.
- **Given** no local server is running, **when** I explicitly start the server, **then** it becomes reachable on loopback and reports useful startup information.
- **Given** a server is reachable at a supplied custom URL, including a custom port, **when** I invoke the client with that URL, **then** it checks that server and reports a successful response without starting a local server.
- **Given** a supplied custom server URL cannot be reached, **when** I invoke the client with that URL, **then** it reports failure with the target and connection context, without starting or switching to a local server.
- **Given** automatic local startup fails, **when** I invoke the default client flow, **then** it reports an actionable startup failure rather than claiming a successful connection.
- **Given** the server receives a health request, **when** I inspect its diagnostic output, **then** I can identify the request and its outcome.
- **Given** the server is running, **when** I request normal shutdown, **then** the process exits cleanly and no longer accepts requests.

Independent verification: exercise automatic local startup, client exit with server survival, reuse of the running server, explicit server startup, successful use of a custom URL/port, custom-URL failure without fallback, and clean server shutdown. Record an actionable local-startup failure using a controlled failure exercise. This exchange must work before the final schema/update-verification slice.

### P3: Maintain the client as reviewed source

As a Ship developer, I want an agent to author and update thin client request methods over shared Rust DTOs so I can develop the API without introducing deterministic client-generation tooling yet.

- **Given** the working server/client exchange, **when** I build from the accepted source, **then** the client compiles and performs the health request using the same shared DTO type as the server, without an AI call or client-generation step during the build.
- **Given** an API declaration changes, **when** an agent updates the affected client methods, **then** the patch is reviewed against the server declaration and exported schema, applicable local checks pass, and the request succeeds against the corresponding server.
- **Given** a shared DTO type changes incompatibly with a consumer, **when** I compile that unchanged consumer, **then** the compiler identifies the incompatible use. This does not claim that every wire or route change is compiler-detected.
- **Given** client/server route or request behavior disagrees, **when** I exercise the affected operation against the corresponding server, **then** the mismatch is exposed rather than inferred safe from a successful build.
- **Given** the current approach becomes costly through recurring drift, repetitive updates, or verification burden, **when** we reconsider deterministic generation, **then** we assess the documented options and obtain approval before changing the workflow.

Independent verification: perform the live health request, inspect direct shared-type use, exercise a temporary API update and an incompatible consumer, and demonstrate a controlled route mismatch through the HTTP boundary. Reconcile the update, repeat the request, then restore the intended baseline and local checks. Record the update instructions and observed evidence, not a promise that an AI produces identical output on repeated runs. No permanent automated test suite is required.

## Non-goals

- CI configuration, deployment workflows, deployment toolchain selection, or cross-platform parity claims.
- Authored automated tests or permanent test scaffolding. Verification uses local checks and manual exercises.
- Terminal hosting, PTYs, terminal emulation, terminal UI, input forwarding, resize handling, or terminal lifecycle APIs. Automatic attachment to or creation of real terminal sessions belongs to the first terminal slice; no placeholder session model is required here.
- SSE delivery, streaming-client generation, compression, snapshot publication, or application-state replication.
- Splits, tabs, persistence, remote access, configuration systems, plugins, or an automation command catalog.
- Non-loopback server listening or a new authentication/access-control policy. A custom client URL does not expand the server's exposure policy or establish production remote-access support.
- A complete future API, additional middleware without a requirement, or general-purpose infrastructure for speculative features.
- Deterministic client generation, generator forks, schema-version conversion, custom templates, or automatic AI generation during builds/runtime in this checkpoint. These remain future options, not implementation tasks.
- Claims of automatic client/schema synchronization or guaranteed compiler detection of route, query, header, status, or serialization drift.

## Functional requirements

- **FR-001:** The repository MUST provide a buildable local foundation for separate server and client execution from the Ship application.
- **FR-002:** The project MUST document the prerequisites and commands needed to build, check, and exercise this checkpoint locally.
- **FR-003:** The project MUST provide reproducible local formatting, linting, and build checks.
- **FR-004:** Command-line help MUST expose the supported server and client operations, default local startup behavior, and custom server URL selection for this checkpoint.
- **FR-005:** The default client flow MUST start a missing local server in the background, reuse an existing local server, and leave the server running after the client exits. Explicit server startup MUST also be supported. The server MUST listen only on loopback for this checkpoint.
- **FR-006:** The client MUST perform a real health request and distinguish success from startup, connection, or request failure. It MUST accept a custom server URL, including a custom port, and use the supplied target without automatically starting or falling back to a different server.
- **FR-007:** Diagnostic output MUST make server startup, request outcomes, and failures observable without obscuring the client's result.
- **FR-008:** Normal server shutdown MUST terminate the process cleanly and stop accepting requests.
- **FR-009:** Each slice MUST pass its applicable local checks and manual acceptance scenarios before work advances to the next slice. The basic health exchange MUST work before the final schema/update-verification slice begins.
- **FR-010:** The final slice MUST demonstrate the reviewed, agent-authored client performing the health request against the running server with direct shared DTO identity.
- **FR-011:** The repository MUST document the agent-assisted client-update workflow. Accepted client source MUST be checked in, editable as ordinary Rust, and built without AI calls or client-generation tooling; no misleading auto-generated/do-not-edit marker.
- **FR-012:** A temporary API-update exercise MUST demonstrate compiler detection of an incompatible shared-type consumer, review of client/server/schema agreement, and live detection of a controlled route mismatch. Reconciled code MUST perform the request against the corresponding server.
- **FR-013:** Deterministic-generation alternatives and their known limitations MUST be recorded for later evaluation. Recurring client drift or maintenance burden prompts a new decision, not silent adoption of a generator, model duplication, or a new build dependency.

## Success criteria

- **SC-001:** The documented local formatting, linting, and build commands all pass on the exercised development environment; prerequisites and any unavailable environment checks are recorded.
- **SC-002:** Manual evidence demonstrates command-line help, automatic local server startup, reuse of an existing server, server survival after client exit, explicit startup, successful health checks, successful custom-URL/port selection, custom-URL failure without local fallback, an actionable local-startup failure, request diagnostics, and clean server shutdown.
- **SC-003:** Verification is recorded for each slice before progression, with the basic server/client exchange demonstrated before schema/update verification.
- **SC-004:** The checked-in client builds without an AI/generator step and successfully calls the running server using shared DTOs. Endpoint methods are separated from startup and transport policy.
- **SC-005:** A temporary API-update exercise demonstrates a compiler-detected incompatible shared-type consumer, an HTTP-observed route mismatch, and a successful call after reconciliation. The intended baseline is restored and its checks pass afterward.
- **SC-006:** No authored automated tests, CI/deployment configuration, terminal implementation, deterministic client-generation tooling, or build/runtime AI calls are introduced by this checkpoint.

Compilation alone is not proof of client/server wire agreement. Agent-authored source is not a successful deterministic-codegen trial; that previous acceptance goal is explicitly superseded.

## Open unknowns

No unresolved product decision in this revision; the revised scope still requires review approval. Architecture and program settle file ownership, schema tooling, routes and update instructions. [architecture.md](architecture.md) records the future-generation options and evidence gaps.

## Superseded decisions

The previously locked codegen trial is retained below as historical evidence. Its requirements, lock status and acceptance claims are superseded by the current revision; do not execute them as current scope.

<details>
<summary>Previous locked product revision: deterministic-codegen trial</summary>

# Initial scaffolding

Status: locked. Approved by Cyan after review. Changes to this paper require renewed approval and reopen any downstream design papers.

## Goal

Give Ship's developers a buildable local foundation with useful checks, observable behavior, and a minimal working server/client exchange before terminal work begins. Finish the checkpoint by proving that API client generation supports a real request and a checked API change, rather than merely producing files.

## User stories

### P1: Develop against a checked foundation

As a Ship developer, I want the project to build and provide reproducible local checks so subsequent slices start from a usable baseline.

- **Given** a checkout and the documented prerequisites, **when** I run the documented local checks, **then** formatting, linting, and build checks complete successfully.
- **Given** the application is built, **when** I request command-line help, **then** I can discover how to start the server and invoke the client.
- **Given** a slice is ready, **when** its verification is recorded, **then** its applicable local checks and manual acceptance scenarios have passed before the next slice begins.

Independent verification: build from the documented prerequisites, run the local checks, and exercise command-line help without needing a running server.

### P2: Exercise an observable local server/client exchange

As a Ship developer, I want to start the server and check it from the client so I can exercise the foundation before adding codegen or terminal behavior.

- **Given** no default local server is running, **when** I invoke the client without a custom server URL, **then** it starts a local server in the background, connects to it, and reports a successful health response.
- **Given** the default local server is already running, **when** I invoke the client without a custom server URL, **then** it uses that server rather than starting another one.
- **Given** the client started the local server, **when** the client exits, **then** the server remains running and a subsequent client invocation can reach it.
- **Given** no local server is running, **when** I explicitly start the server, **then** it becomes reachable on loopback and reports useful startup information.
- **Given** a server is reachable at a supplied custom URL, including a custom port, **when** I invoke the client with that URL, **then** it checks that server and reports a successful response without starting a local server.
- **Given** a supplied custom server URL cannot be reached, **when** I invoke the client with that URL, **then** it reports failure with the target and connection context, without starting or switching to a local server.
- **Given** automatic local startup fails, **when** I invoke the default client flow, **then** it reports an actionable startup failure rather than claiming a successful connection.
- **Given** the server receives a health request, **when** I inspect its diagnostic output, **then** I can identify the request and its outcome.
- **Given** the server is running, **when** I request normal shutdown, **then** the process exits cleanly and no longer accepts requests.

Independent verification: exercise automatic local startup, client exit with server survival, reuse of the running server, explicit server startup, successful use of a custom URL/port, custom-URL failure without fallback, and clean server shutdown. Record an actionable local-startup failure using a controlled failure exercise. This slice must work before codegen is introduced.

### P3: Prove codegen on the working exchange

As a Ship developer, I want a generated client to perform a real API request and reflect API changes so I can decide whether codegen belongs in Ship's foundation.

- **Given** the basic server/client exchange already works, **when** I follow the documented generation procedure, **then** it produces a client that compiles and performs the health request against the running server.
- **Given** the API declaration changes in a way that changes a generated request or response type, **when** I regenerate the client, **then** the generated interface reflects that change and compilation detects incompatible use of the changed interface.
- **Given** the generated interface and caller have been reconciled, **when** I build and repeat the request against the corresponding server, **then** the generated call succeeds.
- **Given** codegen cannot meet these acceptance scenarios or exposes a consequential blocker, **when** that blocker is demonstrated, **then** work stops with the evidence presented to Cyan for a decision; a handwritten client is not silently substituted as the final outcome.

Independent verification: manually generate, compile, and run the client; exercise a temporary API change and its compiler feedback; then restore the intended baseline and repeat the checks. No permanent automated test suite is required.

## Non-goals

- CI configuration, deployment workflows, deployment toolchain selection, or cross-platform parity claims.
- Authored automated tests or permanent test scaffolding. Verification uses local checks and manual exercises.
- Terminal hosting, PTYs, terminal emulation, terminal UI, input forwarding, resize handling, or terminal lifecycle APIs. Automatic attachment to or creation of real terminal sessions belongs to the first terminal slice; no placeholder session model is required here.
- SSE delivery, streaming-client generation, compression, snapshot publication, or application-state replication.
- Splits, tabs, persistence, remote access, configuration systems, plugins, or an automation command catalog.
- Non-loopback server listening or a new authentication/access-control policy. A custom client URL does not expand the server's exposure policy or establish production remote-access support.
- A complete future API, additional middleware without a requirement, or general-purpose infrastructure for speculative features.
- Silently committing Ship to codegen when its trial fails.

## Functional requirements

- **FR-001:** The repository MUST provide a buildable local foundation for separate server and client execution from the Ship application.
- **FR-002:** The project MUST document the prerequisites and commands needed to build, check, and exercise this checkpoint locally.
- **FR-003:** The project MUST provide reproducible local formatting, linting, and build checks.
- **FR-004:** Command-line help MUST expose the supported server and client operations, default local startup behavior, and custom server URL selection for this checkpoint.
- **FR-005:** The default client flow MUST start a missing local server in the background, reuse an existing local server, and leave the server running after the client exits. Explicit server startup MUST also be supported. The server MUST listen only on loopback for this checkpoint.
- **FR-006:** The client MUST perform a real health request and distinguish success from startup, connection, or request failure. It MUST accept a custom server URL, including a custom port, and use the supplied target without automatically starting or falling back to a different server.
- **FR-007:** Diagnostic output MUST make server startup, request outcomes, and failures observable without obscuring the client's result.
- **FR-008:** Normal server shutdown MUST terminate the process cleanly and stop accepting requests.
- **FR-009:** Each slice MUST pass its applicable local checks and manual acceptance scenarios before work advances to the next slice. The basic health exchange MUST work before the final codegen slice begins.
- **FR-010:** The final slice MUST demonstrate a generated client performing the health request against the running server.
- **FR-011:** The generation procedure MUST be documented and repeatable without manually editing generated files.
- **FR-012:** A demonstrated API type change MUST appear in the regenerated interface, and compilation MUST detect incompatible caller use of that changed interface.
- **FR-013:** If codegen cannot satisfy the trial, work MUST stop for Cyan's decision with concrete failure evidence rather than silently replacing the final generated-client outcome.

## Success criteria

- **SC-001:** The documented local formatting, linting, and build commands all pass on the exercised development environment; prerequisites and any unavailable environment checks are recorded.
- **SC-002:** Manual evidence demonstrates command-line help, automatic local server startup, reuse of an existing server, server survival after client exit, explicit startup, successful health checks, successful custom-URL/port selection, custom-URL failure without local fallback, an actionable local-startup failure, request diagnostics, and clean server shutdown.
- **SC-003:** Verification is recorded for each slice before progression, with the basic server/client exchange demonstrated before codegen is added.
- **SC-004:** The generated client builds and successfully calls the running server using the documented generation procedure.
- **SC-005:** A temporary API-change exercise demonstrates regeneration, a compiler-detected incompatible caller, and a successful call after reconciliation. The intended baseline is restored and its checks pass afterward.
- **SC-006:** No authored automated tests, CI/deployment configuration, terminal implementation, or manually edited generated files are introduced by this checkpoint.

A failed codegen trial is decision evidence, not fulfillment of SC-004 or SC-005. The checkpoint remains incomplete unless Cyan explicitly revises its scope through review.

## Open unknowns

None at the product-scope level. Architecture and implementation choices remain for the later design stages; this paper does not select API routes or schemas, generation tools, middleware, lint settings, or dependency delivery mechanisms.

</details>
