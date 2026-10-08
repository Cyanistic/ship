# Spec Delta

## MODIFIED Requirements

### Requirement: Schema inspection without server startup
The server SHALL expose an OpenAPI 3.1 description of every HTTP operation it serves, including health and the `/api/v0` session, tab, pane and attachment operations, together with their shared request and response types. The description SHALL be constructible without a listener, runtime tasks or tracing initialization. The key event inside an input key frame MAY be described only as an object, because its serialization comes from a dependency.

#### Scenario: Inspect the document
- **WHEN** a developer constructs and serializes the API description through the inspection seam
- **THEN** GET `/health`, operation identifier `health`, HTTP 200 and response field types/requiredness agree with the actual route and serialization, without starting a server

#### Scenario: Inspect the session API
- **WHEN** a developer constructs and serializes the API description after the session API is added
- **THEN** every `/api/v0` route appears with its method, path parameters, request body, success and error statuses, and shared schemas whose field names and types agree with actual serialization, including IDs described as kind-prefixed strings

#### Scenario: Inspect the terminal routes
- **WHEN** a developer constructs and serializes the API description after the input and view routes are added
- **THEN** `POST /api/v0/attach/input` and `PUT /api/v0/attach/view` appear with their request and response types, and an input key frame's `key` field is described as an object
