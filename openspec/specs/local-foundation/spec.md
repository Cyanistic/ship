# local-foundation Specification

## Purpose

Provide a usable local Ship executable with discoverable commands, documented prerequisites and repeatable development checks before health and terminal features are added.

## Requirements

### Requirement: Buildable application and command help
The project SHALL provide a buildable Ship executable with help and version output that do not require a running server. Help SHALL describe bare `ship` opening the client and `alt-q` detaching, `ship server status`, explicit server startup with `--starter`, custom target selection, `--config` and `ship config check`, and automatic background-server persistence.

#### Scenario: Foundation help without a server
- **WHEN** a developer builds the foundation and runs `ship --help` or `ship --version` with no server running
- **THEN** the requested output is displayed and the command exits successfully without starting a server

#### Scenario: Command discovery
- **WHEN** a user requests help
- **THEN** help describes `ship`, `ship server status`, `ship --server-url URL`, `ship --config FILE`, `ship config check`, `ship server [--port PORT] [--starter]` and that automatic startup leaves the server running

### Requirement: Repeatable local checks and honest evidence
The project SHALL document repeatable formatting, linting and build commands. Each runnable slice SHALL pass its applicable checks and manual acceptance exercises before progression; unavailable checks SHALL remain explicitly unverified.

#### Scenario: Verify a checkpoint
- **WHEN** a developer completes a runnable slice in the documented development environment
- **THEN** its formatting, linting, debug/release builds and applicable manual exercises are performed and their observed results are recorded before advancing

#### Scenario: Unavailable environment check
- **WHEN** a verification exercise cannot be performed in the available environment
- **THEN** the evidence records the limitation without claiming the behavior or platform passed

### Requirement: End-user documentation
The README SHALL explain what Ship does at this checkpoint, prerequisites, build/run commands, target selection, background startup, log discovery, normal shutdown, the default keys and the config file. For chord spelling it SHALL point to crokey instead of repeating it. Internal implementation/update instructions and slice evidence SHALL remain in development planning rather than the README.

#### Scenario: Run from documented instructions
- **WHEN** a user follows the README for the completed checkpoint
- **THEN** they can build Ship, run the supported workflows, locate an automatically launched server's log and stop the server normally

#### Scenario: Change a key from the README
- **WHEN** a user follows the README's config section to rebind a key
- **THEN** the binding works as described and `ship config check` accepts the file
