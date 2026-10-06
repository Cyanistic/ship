# client-maintenance Specification

## Purpose

Keep Ship's internal client maintainable through reviewed source and shared wire types, while exposing schema descriptions and verifying API updates against the actual server.

## Requirements

### Requirement: Ordinary accepted client source and shared type identity
Endpoint methods SHALL be reviewed, checked-in ordinary Rust using the same shared DTO types as the server, not separately generated equivalents. Normal builds/runtime SHALL require neither model calls nor client generation. Endpoint methods SHALL remain separate from transport and process-startup policy and SHALL NOT be marked auto-generated/do-not-edit.

#### Scenario: Build and use accepted source
- **WHEN** a developer builds the accepted client source and performs health
- **THEN** the request uses the shared response type without an AI call or client-generation step

#### Scenario: Targeted endpoint update
- **WHEN** an agent or person updates an endpoint method
- **THEN** the reviewed patch reuses the shared DTO and preserves transport/startup policy unless a separate change was approved

### Requirement: Schema inspection without server startup
The server SHALL expose an OpenAPI 3.1 description of every HTTP operation it serves, including health and the `/api/v0` session, tab, pane and attachment operations, together with their shared request and response types. The description SHALL be constructible without a listener, runtime tasks or tracing initialization.

#### Scenario: Inspect the document
- **WHEN** a developer constructs and serializes the API description through the inspection seam
- **THEN** GET `/health`, operation identifier `health`, HTTP 200 and response field types/requiredness agree with the actual route and serialization, without starting a server

#### Scenario: Inspect the session API
- **WHEN** a developer constructs and serializes the API description after the session API is added
- **THEN** every `/api/v0` route appears with its method, path parameters, request body, success and error statuses, and shared schemas whose field names and types agree with actual serialization, including IDs described as kind-prefixed strings

### Requirement: Reviewed and exercised client updates
The project SHALL document updates against server declarations, shared DTOs/serialization, schema and existing client conventions. Applicable local checks and requests against the corresponding rebuilt server SHALL verify updates. Compilation alone SHALL NOT be represented as wire agreement or automatic synchronization.

#### Scenario: Incompatible shared-type consumer
- **WHEN** a controlled DTO change leaves a consumer using an incompatible type
- **THEN** compilation exposes that incompatible use, and the reconciled update is verified through a real request

#### Scenario: Compiling route mismatch
- **WHEN** a controlled route change leaves the client using the old path
- **THEN** the request exposes the disagreement even if compilation succeeds, and reconciliation restores successful health

#### Scenario: Restore the accepted baseline
- **WHEN** temporary update exercises finish
- **THEN** the intended source baseline is restored, temporary inspection artifacts are discarded, affected checks/requests are repeated and actual evidence is recorded

### Requirement: Deliberate future generator adoption
The project SHALL preserve evaluated deterministic-generation alternatives and known limitations for later reconsideration. Recurring drift, repetitive changes or verification burden SHALL prompt a reviewed decision, not silent generator adoption or duplicate DTO introduction. File length alone SHALL NOT trigger migration.

#### Scenario: Reconsider the authoring mechanism
- **WHEN** recurring synchronization or maintenance costs justify reviewing the approach
- **THEN** documented candidates and evidence gaps are assessed and approval is obtained before changing client generation or type ownership
