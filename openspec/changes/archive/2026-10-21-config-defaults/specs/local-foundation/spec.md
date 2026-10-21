# Spec Delta

## MODIFIED Requirements

### Requirement: Buildable application and command help
The project SHALL provide a buildable Ship executable with help and version output that do not require a running server. Help SHALL describe bare `ship` opening the client and `alt-q` detaching, `ship server status`, explicit server startup with `--starter`, custom target selection, `--config`, `ship config check` and `ship config defaults`, and automatic background-server persistence.

#### Scenario: Foundation help without a server
- **WHEN** a developer builds the foundation and runs `ship --help` or `ship --version` with no server running
- **THEN** the requested output is displayed and the command exits successfully without starting a server

#### Scenario: Command discovery
- **WHEN** a user requests help
- **THEN** help describes `ship`, `ship server status`, `ship --server-url URL`, `ship --config FILE`, `ship config check`, `ship config defaults`, `ship server [--port PORT] [--starter]` and that automatic startup leaves the server running
