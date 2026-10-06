# Proposal

## Why

Ship needs a buildable, observable local foundation before terminal work begins. A minimal health exchange will establish crate boundaries, local-server lifecycle and a maintainable shared-type client without prematurely adopting code-generation tooling.

## What Changes

- Establish the four-crate Cargo workspace, contextual errors, local checks and end-user documentation.
- Make bare `ship` perform the health exchange and `ship server` run an explicit foreground loopback server.
- Automatically launch a missing default-local server, reuse a compatible running one, and leave it alive after client exit.
- Support explicit custom server URLs without local startup or fallback.
- Provide useful startup/request/failure diagnostics and signal-based graceful shutdown.
- Check in thin agent-authored client methods using shared DTOs directly, with transport and startup policy kept separate.
- Describe the server API using Utoipa 6/OpenAPI 3.1 and exercise the reviewed client-update workflow.
- Deliver three independently runnable, verified checkpoints: foundation, health/lifecycle, then schema/update verification.

No existing supported API is removed. The abandoned partial skeleton is incomplete, unverified input to reconcile later, not an accepted implementation.

## Capabilities

### New Capabilities

- `local-foundation`: A buildable application, discoverable help/version, reproducible local checks and user documentation.
- `health-exchange`: The health contract, target selection, automatic local launch/reuse, diagnostics and server shutdown.
- `client-maintenance`: Direct shared DTO use, ordinary checked-in endpoint source, pure schema construction and an exercised API-update workflow.

### Modified Capabilities

None. The project's main spec inventory is empty.

## Impact

Implementation will affect the root Cargo workspace, Cargo-owned lockfile, README, and `ship`, `ship-core`, `ship-client`, `ship-server` crates. Runtime dependencies include Clap, Tokio, Axum, Reqwest, Serde, tracing, URL, nix and tempfile; Utoipa enters the final slice. Use ordinary compatible version constraints, not exact pins.

Planning follows the active product and architecture papers plus the latest program review: bare-command client workflow, unsafe permitted, plain DTOs and end-user README. Historical codegen sections and older paper snippets are not implementation instructions.

The `err!` syntax is grounded in Cyan's supplied reference error module, adapted to Ship's smaller error representation. No CI, deployment, permanent authored tests, terminal/SSE/compression code, deterministic generator, model invocation, experiment integration or new access-control policy is included. Artifact creation does not start implementation.
